//! GitHub sign-in and the calls Obsidian makes as the signed-in player: issue reports and their
//! replies, stars, gists for shared profiles and sync, and the pull request of a Store submission.
//!
//! Sign-in is GitHub's device flow: Obsidian asks for a code, the player types it at
//! github.com/login/device, and Obsidian polls until GitHub hands over a token. The OAuth app needs
//! no secret for that, so `CLIENT_ID` is public. The token stays in `<data>/github.json` and in
//! memory (`token`), where `updates::GitHub` also picks it up for the higher API limit.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{ObsidianError, Result};

/// The OAuth app "Obsidian Launcher" (device flow on, tokens that don't expire: renewing one would
/// need the app's secret, which a desktop app can't keep).
pub const CLIENT_ID: &str = "Ov23lihypHqV95HkfJNN";
const FILE: &str = "github.json";
const REPORTS: &str = "github-reports.json";
const API: &str = "https://api.github.com";
const WEB: &str = "https://github.com";
/// What reporting, stars and the Store submission need.
pub const BASE_SCOPES: &[&str] = &["public_repo"];
/// Sync between PCs.
pub const GIST: &str = "gist";
/// Early access: private repositories, which GitHub only grants with read and write to all of them.
pub const PRIVATE: &str = "repo";
/// The description that marks the sync gist among the player's gists.
pub const SYNC_GIST: &str = "Obsidian Launcher sync";

static TOKEN: RwLock<Option<String>> = RwLock::new(None);

/// The token of the signed-in player, for any GitHub API call.
pub fn token() -> Option<String> {
    TOKEN.read().ok().and_then(|t| t.clone())
}

fn set_token(t: Option<String>) {
    if let Ok(mut slot) = TOKEN.write() {
        *slot = t;
    }
}

/// The account as `<data>/github.json` keeps it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    pub token: String,
    pub login: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    /// The sync gist, once found or made.
    #[serde(default)]
    pub sync_gist: Option<String>,
}

/// The account as the window sees it: never the token.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub login: String,
    pub avatar: Option<String>,
    pub scopes: Vec<String>,
    pub sync_gist: Option<String>,
}

impl From<&Saved> for Account {
    fn from(s: &Saved) -> Self {
        Account { login: s.login.clone(), avatar: s.avatar.clone(), scopes: s.scopes.clone(), sync_gist: s.sync_gist.clone() }
    }
}

fn path(data: &Path) -> PathBuf {
    data.join(FILE)
}

pub fn saved(data: &Path) -> Option<Saved> {
    crate::paths::json_file::load::<Saved>(&path(data)).filter(|s| !s.token.is_empty())
}

fn save(data: &Path, s: &Saved) -> Result<()> {
    crate::paths::write_atomic(&path(data), &serde_json::to_vec_pretty(s)?)?;
    Ok(())
}

/// Reads the saved account into memory, at startup. The account, if any.
pub fn load(data: &Path) -> Option<Account> {
    let s = saved(data);
    set_token(s.as_ref().map(|s| s.token.clone()));
    s.as_ref().map(Account::from)
}

/// Forgets the account on this PC. The grant itself stays on GitHub until the player revokes it
/// there (`revoke_url`).
pub fn sign_out(data: &Path) -> Result<()> {
    set_token(None);
    match std::fs::remove_file(path(data)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Where the player revokes Obsidian's access on GitHub.
pub fn revoke_url() -> String {
    if CLIENT_ID.is_empty() { format!("{WEB}/settings/applications") } else { format!("{WEB}/settings/connections/applications/{CLIENT_ID}") }
}

fn gh_err(e: impl std::fmt::Display) -> ObsidianError {
    ObsidianError::UpdateNetwork(e.to_string())
}

fn refused(status: u16, body: &str) -> ObsidianError {
    let msg = serde_json::from_str::<Value>(body).ok().and_then(|v| v["message"].as_str().map(str::to_string)).unwrap_or_default();
    match status {
        401 => ObsidianError::GitHubSignIn,
        _ => ObsidianError::GitHub(if msg.is_empty() { format!("GitHub answered {status}") } else { format!("{msg} ({status})") }),
    }
}

/// A repository name as `owner/name`, checked so it can't leave the API path.
pub fn repo_name(s: &str) -> Option<String> {
    let s = s.trim().trim_end_matches('/');
    let s = s.strip_prefix("https://github.com/").or_else(|| s.strip_prefix("http://github.com/")).or_else(|| s.strip_prefix("github.com/")).unwrap_or(s);
    let s = s.strip_suffix(".git").unwrap_or(s);
    let mut parts = s.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let fits = |p: &str| !p.is_empty() && p.len() <= 100 && !p.starts_with('.') && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
    (fits(owner) && fits(name)).then(|| format!("{owner}/{name}"))
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

/// Calls to GitHub, as the signed-in player when there is a token.
pub struct Client {
    api: String,
    web: String,
    token: Option<String>,
    agent: ureq::Agent,
}

/// One answer: the status, the body, and the scopes GitHub says the token has.
pub struct Answer {
    pub status: u16,
    pub body: String,
    pub scopes: Option<String>,
}

impl Answer {
    pub fn json(&self) -> Result<Value> {
        Ok(serde_json::from_str(&self.body)?)
    }
    fn ok(self) -> Result<Answer> {
        if (200..300).contains(&self.status) { Ok(self) } else { Err(refused(self.status, &self.body)) }
    }
}

impl Client {
    /// The real GitHub with the saved token, or `OBSIDIAN_GITHUB_API` (tests) for both sites.
    pub fn new() -> Self {
        match std::env::var("OBSIDIAN_GITHUB_API") {
            Ok(base) => Client::at(&base, &base, token()),
            Err(_) => Client::at(API, WEB, token()),
        }
    }

    pub fn at(api: &str, web: &str, token: Option<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .user_agent(concat!("Obsidian/", env!("CARGO_PKG_VERSION")))
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .timeout_recv_body(Some(Duration::from_secs(120)))
            .build()
            .into();
        Client { api: api.trim_end_matches('/').to_string(), web: web.trim_end_matches('/').to_string(), token, agent }
    }

    pub fn signed_in(&self) -> bool {
        self.token.is_some()
    }

    fn need_token(&self) -> Result<()> {
        if self.token.is_some() { Ok(()) } else { Err(ObsidianError::GitHubSignIn) }
    }

    /// One API call. `path` starts with `/`; `body` goes as JSON.
    pub fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Answer> {
        self.call_accept(method, path, body, "application/vnd.github+json")
    }

    fn call_accept(&self, method: &str, path: &str, body: Option<&Value>, accept: &str) -> Result<Answer> {
        let url = format!("{}{path}", self.api);
        let auth = self.token.as_ref().map(|t| format!("Bearer {t}"));
        let bytes = body.map(serde_json::to_vec).transpose()?;
        macro_rules! headers {
            ($req:expr) => {{
                let r = $req.header("Accept", accept).header("X-GitHub-Api-Version", "2022-11-28");
                match &auth {
                    Some(a) => r.header("Authorization", a.as_str()),
                    None => r,
                }
            }};
        }
        let res = match method {
            "GET" => headers!(self.agent.get(&url)).call(),
            "DELETE" => headers!(self.agent.delete(&url)).call(),
            m => {
                let req = match m {
                    "POST" => self.agent.post(&url),
                    "PUT" => self.agent.put(&url),
                    "PATCH" => self.agent.patch(&url),
                    other => return Err(ObsidianError::GitHub(format!("unknown method {other}"))),
                };
                let req = headers!(req);
                match &bytes {
                    Some(b) => req.header("Content-Type", "application/json").send(&b[..]),
                    None => req.send_empty(),
                }
            }
        };
        let mut res = res.map_err(gh_err)?;
        let status = res.status().as_u16();
        let scopes = res.headers().get("x-oauth-scopes").and_then(|v| v.to_str().ok()).map(str::to_string);
        let body = res.body_mut().with_config().limit(16 << 20).read_to_string().map_err(gh_err)?;
        Ok(Answer { status, body, scopes })
    }

    /// A file's text through the API (`Accept: raw`), or None when there is no such file.
    pub fn call_raw(&self, path: &str) -> Result<Option<String>> {
        let a = self.call_accept("GET", path, None, "application/vnd.github.raw+json")?;
        if a.status == 404 {
            return Ok(None);
        }
        Ok(Some(a.ok()?.body))
    }

    fn get(&self, path: &str) -> Result<Value> {
        self.call("GET", path, None)?.ok()?.json()
    }

    fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<Value> {
        let url = format!("{}{path}", self.web);
        let body: Vec<String> = form.iter().map(|(k, v)| format!("{k}={}", enc(v))).collect();
        let mut res = self
            .agent
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(body.join("&"))
            .map_err(gh_err)?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().map_err(gh_err)?;
        if !(200..300).contains(&status) {
            return Err(refused(status, &text));
        }
        Ok(serde_json::from_str(&text)?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}

// ---- Sign-in ----

/// The code the player types at `verification_uri`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

/// Starts a sign-in asking for `scopes`.
pub fn device_start(c: &Client, scopes: &[String]) -> Result<DeviceCode> {
    if CLIENT_ID.is_empty() {
        return Err(ObsidianError::GitHub("this build of Obsidian has no GitHub app set up".into()));
    }
    let v = c.post_form("/login/device/code", &[("client_id", CLIENT_ID), ("scope", &scopes.join(" "))])?;
    let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    if s("device_code").is_empty() {
        return Err(ObsidianError::GitHub(s("error_description")));
    }
    Ok(DeviceCode {
        device_code: s("device_code"),
        user_code: s("user_code"),
        verification_uri: s("verification_uri"),
        interval: v["interval"].as_u64().unwrap_or(5),
        expires_in: v["expires_in"].as_u64().unwrap_or(900),
    })
}

/// Where a sign-in stands after one poll.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Poll {
    /// Not entered yet: ask again after `interval` seconds.
    Pending { interval: u64 },
    Done { account: Account },
    Denied,
    Expired,
}

/// Asks once whether the player entered the code. On success the account is saved and in use.
pub fn device_poll(data: &Path, c: &Client, device_code: &str, interval: u64) -> Result<Poll> {
    let v = c.post_form(
        "/login/oauth/access_token",
        &[("client_id", CLIENT_ID), ("device_code", device_code), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code")],
    )?;
    match v["error"].as_str() {
        Some("authorization_pending") => return Ok(Poll::Pending { interval }),
        Some("slow_down") => return Ok(Poll::Pending { interval: v["interval"].as_u64().unwrap_or(interval + 5) }),
        Some("access_denied") => return Ok(Poll::Denied),
        Some("expired_token") => return Ok(Poll::Expired),
        Some(other) => return Err(ObsidianError::GitHub(v["error_description"].as_str().unwrap_or(other).to_string())),
        None => {}
    }
    let token = v["access_token"].as_str().unwrap_or_default().to_string();
    if token.is_empty() {
        return Err(ObsidianError::GitHub("GitHub sent no token".into()));
    }
    let me = Client::at(&c.api, &c.web, Some(token.clone()));
    let answer = me.call("GET", "/user", None)?.ok()?;
    let user = answer.json()?;
    let scopes = answer
        .scopes
        .clone()
        .or_else(|| v["scope"].as_str().map(|s| s.replace(',', ", ")))
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let login = user["login"].as_str().unwrap_or_default().to_string();
    // A sign-in for more access keeps the sync gist already found for the same account.
    let sync_gist = saved(data).filter(|s| s.login == login).and_then(|s| s.sync_gist);
    let s = Saved { token: token.clone(), login, avatar: user["avatar_url"].as_str().map(str::to_string), scopes, sync_gist };
    save(data, &s)?;
    set_token(Some(token));
    Ok(Poll::Done { account: Account::from(&s) })
}

/// Checks the saved token with GitHub: the account as it is now, or None (and signed out) when
/// GitHub no longer accepts the token.
pub fn verify(data: &Path, c: &Client) -> Result<Option<Account>> {
    let Some(mut s) = saved(data) else { return Ok(None) };
    let answer = c.call("GET", "/user", None)?;
    if answer.status == 401 {
        sign_out(data)?;
        return Ok(None);
    }
    let answer = answer.ok()?;
    let user = answer.json()?;
    s.login = user["login"].as_str().unwrap_or(&s.login).to_string();
    s.avatar = user["avatar_url"].as_str().map(str::to_string).or(s.avatar);
    if let Some(sc) = &answer.scopes {
        s.scopes = sc.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
    }
    save(data, &s)?;
    Ok(Some(Account::from(&s)))
}

/// Whether the saved token was granted `scope` (`repo` covers `public_repo`).
pub fn has_scope(data: &Path, scope: &str) -> bool {
    saved(data).is_some_and(|s| s.scopes.iter().any(|x| x == scope || (scope == "public_repo" && x == "repo")))
}

// ---- Repositories ----

/// What the mod page needs to know of a repository, as the player sees it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub full_name: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub has_issues: bool,
    pub stars: u64,
    pub owner: String,
    pub owner_is_org: bool,
    /// The player may push: an author of the mod.
    pub push: bool,
    pub default_branch: String,
    pub open_issues: u64,
}

fn repo_info(v: &Value) -> RepoInfo {
    RepoInfo {
        full_name: v["full_name"].as_str().unwrap_or_default().to_string(),
        private: v["private"].as_bool().unwrap_or(false),
        fork: v["fork"].as_bool().unwrap_or(false),
        archived: v["archived"].as_bool().unwrap_or(false),
        has_issues: v["has_issues"].as_bool().unwrap_or(false),
        stars: v["stargazers_count"].as_u64().unwrap_or(0),
        owner: v["owner"]["login"].as_str().unwrap_or_default().to_string(),
        owner_is_org: v["owner"]["type"].as_str() == Some("Organization"),
        push: v["permissions"]["push"].as_bool().unwrap_or(false),
        default_branch: v["default_branch"].as_str().unwrap_or("main").to_string(),
        open_issues: v["open_issues_count"].as_u64().unwrap_or(0),
    }
}

/// The repository, or None when it doesn't exist or the player can't see it.
pub fn repo(c: &Client, repo: &str) -> Result<Option<RepoInfo>> {
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(format!("{repo} is not a GitHub repository")))?;
    let a = c.call("GET", &format!("/repos/{r}"), None)?;
    if a.status == 404 {
        return Ok(None);
    }
    Ok(Some(repo_info(&a.ok()?.json()?)))
}

// ---- Stars ----

pub fn starred(c: &Client, repo: &str) -> Result<bool> {
    c.need_token()?;
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    let a = c.call("GET", &format!("/user/starred/{r}"), None)?;
    match a.status {
        204 => Ok(true),
        404 => Ok(false),
        s => Err(refused(s, &a.body)),
    }
}

/// Every repository the player starred, as `owner/name` in lower case (up to 1,000).
pub fn starred_repos(c: &Client) -> Result<Vec<String>> {
    c.need_token()?;
    let mut out = Vec::new();
    for page in 1..=10 {
        let list = c.get(&format!("/user/starred?per_page=100&page={page}"))?;
        let items = list.as_array().cloned().unwrap_or_default();
        out.extend(items.iter().filter_map(|r| r["full_name"].as_str().map(str::to_lowercase)));
        if items.len() < 100 {
            break;
        }
    }
    Ok(out)
}

pub fn set_star(c: &Client, repo: &str, on: bool) -> Result<()> {
    c.need_token()?;
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    c.call(if on { "PUT" } else { "DELETE" }, &format!("/user/starred/{r}"), None)?.ok()?;
    Ok(())
}

// ---- Issue reports ----

/// The issue templates of a repository, as text for the window to read (YAML forms or Markdown with
/// front matter), and whether a blank issue is allowed.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Templates {
    pub files: Vec<TemplateFile>,
    pub blank: bool,
    pub has_issues: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateFile {
    pub name: String,
    pub text: String,
}

pub fn templates(c: &Client, repo: &str) -> Result<Templates> {
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    let info = self::repo(c, &r)?.ok_or_else(|| ObsidianError::GitHub(format!("{r} was not found")))?;
    let mut out = Templates { files: Vec::new(), blank: true, has_issues: info.has_issues && !info.archived };
    if !out.has_issues {
        return Ok(out);
    }
    let list = c.call("GET", &format!("/repos/{r}/contents/.github/ISSUE_TEMPLATE"), None)?;
    if list.status == 404 {
        return Ok(out);
    }
    let list = list.ok()?.json()?;
    for f in list.as_array().into_iter().flatten().take(20) {
        let name = f["name"].as_str().unwrap_or_default();
        let lower = name.to_lowercase();
        if f["type"].as_str() != Some("file") || !(lower.ends_with(".yml") || lower.ends_with(".yaml") || lower.ends_with(".md")) {
            continue;
        }
        if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_. ".contains(&b)) {
            continue;
        }
        let a = c.call_accept("GET", &format!("/repos/{r}/contents/.github/ISSUE_TEMPLATE/{}", enc(name)), None, "application/vnd.github.raw+json")?;
        if a.status != 200 {
            continue;
        }
        if lower == "config.yml" || lower == "config.yaml" {
            // Only this line matters here; the rest of the file is for github.com.
            out.blank = !a.body.lines().any(|l| {
                let l = l.trim().to_lowercase();
                l.starts_with("blank_issues_enabled:") && l.contains("false")
            });
            continue;
        }
        out.files.push(TemplateFile { name: name.to_string(), text: a.body });
    }
    out.files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// One report sent from Obsidian, kept so the player can follow it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub repo: String,
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub mod_id: String,
    #[serde(default)]
    pub mod_name: String,
    pub url: String,
    pub created: String,
    #[serde(default)]
    pub state: String,
    /// Comments on the issue now, and when the player last read them.
    #[serde(default)]
    pub comments: u64,
    #[serde(default)]
    pub seen: u64,
    #[serde(default)]
    pub updated: String,
}

pub fn reports(data: &Path) -> Vec<Report> {
    crate::paths::json_file::load(&data.join(REPORTS)).unwrap_or_default()
}

fn save_reports(data: &Path, list: &[Report]) -> Result<()> {
    crate::paths::write_atomic(&data.join(REPORTS), &serde_json::to_vec_pretty(list)?)?;
    Ok(())
}

/// Files the issue as the player and keeps it in the list of reports.
pub fn create_issue(data: &Path, c: &Client, repo: &str, title: &str, body: &str, labels: &[String], mod_id: &str, mod_name: &str) -> Result<Report> {
    c.need_token()?;
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    let (title, body) = (title.trim(), body.trim());
    if title.chars().count() < 4 || title.chars().count() > 200 {
        return Err(ObsidianError::GitHub("the title needs 4 to 200 characters".into()));
    }
    if body.len() > 60_000 {
        return Err(ObsidianError::GitHub("the report is too long".into()));
    }
    // GitHub drops labels the player may not set, so asking costs nothing.
    let labels: Vec<&String> = labels.iter().filter(|l| !l.trim().is_empty()).take(10).collect();
    let v = c.call("POST", &format!("/repos/{r}/issues"), Some(&json!({ "title": title, "body": body, "labels": labels })))?.ok()?.json()?;
    let rep = Report {
        repo: r,
        number: v["number"].as_u64().unwrap_or(0),
        title: title.to_string(),
        mod_id: mod_id.to_string(),
        mod_name: mod_name.to_string(),
        url: v["html_url"].as_str().unwrap_or_default().to_string(),
        created: v["created_at"].as_str().unwrap_or_default().to_string(),
        state: "open".into(),
        comments: 0,
        seen: 0,
        updated: v["updated_at"].as_str().unwrap_or_default().to_string(),
    };
    let mut list = reports(data);
    list.insert(0, rep.clone());
    save_reports(data, &list)?;
    Ok(rep)
}

/// Asks GitHub where each report stands now. A report that can't be read keeps what was known.
pub fn refresh_reports(data: &Path, c: &Client) -> Result<Vec<Report>> {
    let mut list = reports(data);
    for r in &mut list {
        let Ok(a) = c.call("GET", &format!("/repos/{}/issues/{}", r.repo, r.number), None) else { continue };
        if a.status == 404 || a.status == 410 {
            r.state = "deleted".into();
            continue;
        }
        let Ok(v) = a.ok().and_then(|a| a.json()) else { continue };
        r.state = if v["state_reason"].as_str() == Some("completed") { "fixed".into() } else { v["state"].as_str().unwrap_or("open").to_string() };
        r.comments = v["comments"].as_u64().unwrap_or(r.comments);
        r.updated = v["updated_at"].as_str().unwrap_or_default().to_string();
        r.title = v["title"].as_str().unwrap_or(&r.title).to_string();
    }
    save_reports(data, &list)?;
    Ok(list)
}

/// Takes a report off the list on this PC. The issue stays on GitHub.
pub fn forget_report(data: &Path, repo: &str, number: u64) -> Result<Vec<Report>> {
    let mut list = reports(data);
    list.retain(|r| !(r.repo == repo && r.number == number));
    save_reports(data, &list)?;
    Ok(list)
}

/// One message of an issue: the report itself first, then the replies.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub author: String,
    pub avatar: Option<String>,
    /// OWNER, MEMBER, COLLABORATOR, CONTRIBUTOR, NONE...: whether the author speaks for the mod.
    pub association: String,
    pub body: String,
    pub created: String,
}

fn message(v: &Value) -> Message {
    Message {
        author: v["user"]["login"].as_str().unwrap_or_default().to_string(),
        avatar: v["user"]["avatar_url"].as_str().map(str::to_string),
        association: v["author_association"].as_str().unwrap_or("NONE").to_string(),
        body: v["body"].as_str().unwrap_or_default().to_string(),
        created: v["created_at"].as_str().unwrap_or_default().to_string(),
    }
}

/// The thread of an issue, marked as read when it is one of the player's reports.
pub fn thread(data: &Path, c: &Client, repo: &str, number: u64) -> Result<Vec<Message>> {
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    let issue = c.get(&format!("/repos/{r}/issues/{number}"))?;
    let mut out = vec![message(&issue)];
    let mut page = 1;
    loop {
        let list = c.get(&format!("/repos/{r}/issues/{number}/comments?per_page=100&page={page}"))?;
        let items = list.as_array().cloned().unwrap_or_default();
        out.extend(items.iter().map(message));
        if items.len() < 100 || page >= 10 {
            break;
        }
        page += 1;
    }
    let mut list = reports(data);
    if let Some(rep) = list.iter_mut().find(|x| x.repo == r && x.number == number) {
        rep.comments = (out.len() - 1) as u64;
        rep.seen = rep.comments;
        save_reports(data, &list)?;
    }
    Ok(out)
}

pub fn reply(data: &Path, c: &Client, repo: &str, number: u64, body: &str) -> Result<Vec<Message>> {
    c.need_token()?;
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    if body.trim().is_empty() {
        return Err(ObsidianError::GitHub("the reply is empty".into()));
    }
    c.call("POST", &format!("/repos/{r}/issues/{number}/comments"), Some(&json!({ "body": body.trim() })))?.ok()?;
    thread(data, c, &r, number)
}

/// An open issue of a mod, for its authors.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenIssue {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub comments: u64,
    pub created: String,
    pub url: String,
    pub labels: Vec<String>,
}

pub fn open_issues(c: &Client, repo: &str) -> Result<Vec<OpenIssue>> {
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    let list = c.get(&format!("/repos/{r}/issues?state=open&per_page=50&sort=created"))?;
    Ok(list
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| v.get("pull_request").is_none())
        .map(|v| OpenIssue {
            number: v["number"].as_u64().unwrap_or(0),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            author: v["user"]["login"].as_str().unwrap_or_default().to_string(),
            comments: v["comments"].as_u64().unwrap_or(0),
            created: v["created_at"].as_str().unwrap_or_default().to_string(),
            url: v["html_url"].as_str().unwrap_or_default().to_string(),
            labels: v["labels"].as_array().into_iter().flatten().filter_map(|l| l["name"].as_str().map(str::to_string)).collect(),
        })
        .collect())
}

// ---- Gists ----

/// A new gist with `files` (name to text). Returns the id and the address.
pub fn create_gist(c: &Client, description: &str, public: bool, files: &BTreeMap<String, String>) -> Result<(String, String)> {
    c.need_token()?;
    let files: serde_json::Map<String, Value> = files.iter().map(|(k, v)| (k.clone(), json!({ "content": v }))).collect();
    let v = c.call("POST", "/gists", Some(&json!({ "description": description, "public": public, "files": files })))?.ok()?.json()?;
    Ok((v["id"].as_str().unwrap_or_default().to_string(), v["html_url"].as_str().unwrap_or_default().to_string()))
}

pub fn update_gist(c: &Client, id: &str, files: &BTreeMap<String, String>) -> Result<()> {
    c.need_token()?;
    let files: serde_json::Map<String, Value> = files.iter().map(|(k, v)| (k.clone(), json!({ "content": v }))).collect();
    c.call("PATCH", &format!("/gists/{id}"), Some(&json!({ "files": files })))?.ok()?;
    Ok(())
}

/// The files of a gist, and when it last changed. None when it doesn't exist.
pub fn read_gist(c: &Client, id: &str) -> Result<Option<(BTreeMap<String, String>, String)>> {
    let a = c.call("GET", &format!("/gists/{id}"), None)?;
    if a.status == 404 {
        return Ok(None);
    }
    let v = a.ok()?.json()?;
    let mut files = BTreeMap::new();
    for (name, f) in v["files"].as_object().into_iter().flatten() {
        // GitHub cuts the content of big files and gives the raw address instead.
        let text = if f["truncated"].as_bool() == Some(true) {
            let raw = f["raw_url"].as_str().unwrap_or_default();
            if !raw.starts_with("https://gist.githubusercontent.com/") {
                continue;
            }
            let mut res = c.agent.get(raw).call().map_err(gh_err)?;
            res.body_mut().with_config().limit(8 << 20).read_to_string().map_err(gh_err)?
        } else {
            f["content"].as_str().unwrap_or_default().to_string()
        };
        files.insert(name.clone(), text);
    }
    Ok(Some((files, v["updated_at"].as_str().unwrap_or_default().to_string())))
}

/// The player's sync gist: the one saved, else the first with `SYNC_GIST` as its description.
pub fn find_sync_gist(data: &Path, c: &Client) -> Result<Option<String>> {
    c.need_token()?;
    let mut s = saved(data).ok_or(ObsidianError::GitHubSignIn)?;
    if let Some(id) = &s.sync_gist
        && c.call("GET", &format!("/gists/{id}"), None)?.status == 200
    {
        return Ok(Some(id.clone()));
    }
    for page in 1..=10 {
        let list = c.get(&format!("/gists?per_page=100&page={page}"))?;
        let items = list.as_array().cloned().unwrap_or_default();
        if let Some(g) = items.iter().find(|g| g["description"].as_str() == Some(SYNC_GIST)) {
            s.sync_gist = g["id"].as_str().map(str::to_string);
            save(data, &s)?;
            return Ok(s.sync_gist);
        }
        if items.len() < 100 {
            break;
        }
    }
    s.sync_gist = None;
    save(data, &s)?;
    Ok(None)
}

pub fn remember_sync_gist(data: &Path, id: &str) -> Result<()> {
    let mut s = saved(data).ok_or(ObsidianError::GitHubSignIn)?;
    s.sync_gist = Some(id.to_string());
    save(data, &s)
}

// ---- Store submission ----

/// The index repository a submission goes to.
pub const INDEX_REPO: &str = "Alomare/ObsidianStoreIndex";

/// Opens the pull request that adds `repo` to `mods` in the index's mods.json, from a fork in the
/// player's account. Returns the address of the pull request.
pub fn open_submission(c: &Client, repo: &str, note: &str) -> Result<String> {
    c.need_token()?;
    let r = repo_name(repo).ok_or_else(|| ObsidianError::GitHub(repo.to_string()))?;
    let me = c.get("/user")?["login"].as_str().unwrap_or_default().to_string();
    // The fork: made once, then brought up to date with the index.
    let fork = c.call("POST", &format!("/repos/{INDEX_REPO}/forks"), Some(&json!({ "default_branch_only": true })))?.ok()?.json()?;
    let fork_name = fork["full_name"].as_str().unwrap_or_default().to_string();
    let mut ready = false;
    for _ in 0..20 {
        if c.call("GET", &format!("/repos/{fork_name}/branches/main"), None)?.status == 200 {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    if !ready {
        return Err(ObsidianError::GitHub("the fork of the Store index is not ready yet: try again in a minute".into()));
    }
    let _ = c.call("POST", &format!("/repos/{fork_name}/merge-upstream"), Some(&json!({ "branch": "main" })));
    let upstream = c.get(&format!("/repos/{INDEX_REPO}/contents/mods.json?ref=main"))?;
    let text = decode_b64(upstream["content"].as_str().unwrap_or_default())?;
    let new_text = add_to_mods_json(&text, &r)?;
    let head = c.get(&format!("/repos/{fork_name}/git/ref/heads/main"))?;
    let sha = head["object"]["sha"].as_str().unwrap_or_default();
    let branch = format!("add-{}", r.to_lowercase().replace('/', "-"));
    let made = c.call("POST", &format!("/repos/{fork_name}/git/refs"), Some(&json!({ "ref": format!("refs/heads/{branch}"), "sha": sha })))?;
    if made.status == 422 {
        // Left from an earlier try: start it again from main.
        c.call("PATCH", &format!("/repos/{fork_name}/git/refs/heads/{branch}"), Some(&json!({ "sha": sha, "force": true })))?.ok()?;
    } else {
        made.ok()?;
    }
    let in_fork = c.get(&format!("/repos/{fork_name}/contents/mods.json?ref={branch}"))?;
    c.call(
        "PUT",
        &format!("/repos/{fork_name}/contents/mods.json"),
        Some(&json!({
            "message": format!("Add {r}"),
            "content": encode_b64(new_text.as_bytes()),
            "sha": in_fork["sha"],
            "branch": branch,
        })),
    )?
    .ok()?;
    let body = format!(
        "Adds https://github.com/{r} to the Store.\n\nSent from Obsidian Launcher, which checked the repository first.{}",
        if note.trim().is_empty() { String::new() } else { format!("\n\n{}", note.trim()) }
    );
    let pr = c.call(
        "POST",
        &format!("/repos/{INDEX_REPO}/pulls"),
        Some(&json!({ "title": format!("Add {r}"), "head": format!("{me}:{branch}"), "base": "main", "body": body, "maintainer_can_modify": true })),
    )?;
    if pr.status == 422 && pr.body.contains("already exists") {
        return Err(ObsidianError::GitHub("a pull request for this mod is already open".into()));
    }
    Ok(pr.ok()?.json()?["html_url"].as_str().unwrap_or_default().to_string())
}

/// mods.json with `repo` added at the end of `mods`, keeping the file's layout of one name a line.
pub fn add_to_mods_json(text: &str, repo: &str) -> Result<String> {
    let mut v: Value = serde_json::from_str(text)?;
    let lower = repo.to_lowercase();
    let listed = ["kit", "mods"].iter().any(|k| v[k].as_array().into_iter().flatten().any(|x| x.as_str().map(str::to_lowercase).as_deref() == Some(&lower)));
    if listed {
        return Err(ObsidianError::GitHub(format!("{repo} is already in the Store")));
    }
    v["mods"].as_array_mut().ok_or_else(|| ObsidianError::GitHub("mods.json has no mods list".into()))?.push(json!(repo));
    // Insert into the text so the rest of the file (key order, layout) stays as the index has it;
    // a re-serialised Value would sort the keys and make every submission diff noisy.
    if let Some(out) = insert_into_mods(text, repo) {
        if serde_json::from_str::<Value>(&out).ok().as_ref() == Some(&v) {
            return Ok(out);
        }
    }
    let mut out = serde_json::to_string_pretty(&v)?;
    out.push('\n');
    Ok(out)
}

/// Appends `repo` as the last entry of the top-level "mods" array, copying the indent of the entry
/// before it. None when the array cannot be found in the text.
fn insert_into_mods(text: &str, repo: &str) -> Option<String> {
    let key = text.find("\"mods\"")?;
    let after_key = &text[key + 6..];
    let colon = after_key.find(|c: char| !c.is_whitespace())?;
    if !after_key[colon..].starts_with(':') {
        return None;
    }
    let open = key + 6 + colon + 1 + after_key[colon + 1..].find(|c: char| !c.is_whitespace())?;
    if text.as_bytes()[open] != b'[' {
        return None;
    }
    // The closing bracket, skipping over strings.
    let mut in_str = false;
    let mut escaped = false;
    let mut close = None;
    for (i, c) in text[open + 1..].char_indices() {
        if in_str {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_str = false,
                _ => {}
            }
        } else if c == '"' {
            in_str = true;
        } else if c == ']' {
            close = Some(open + 1 + i);
            break;
        }
    }
    let close = close?;
    let inner = &text[open + 1..close];
    let entry = serde_json::to_string(repo).ok()?;
    let Some(last) = inner.trim_end().len().checked_sub(1).map(|n| open + 1 + n + 1) else {
        return Some(format!("{}{}{}", &text[..open + 1], entry, &text[close..]));
    };
    let line_start = text[..last].rfind('\n').map_or(0, |n| n + 1);
    let indent: String = text[line_start..].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let sep = if inner.contains('\n') { format!(",\n{indent}") } else { ", ".to_string() };
    Some(format!("{}{sep}{entry}{}", &text[..last], &text[last..]))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode_b64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn decode_b64(text: &str) -> Result<String> {
    let mut bits = 0u32;
    let mut n = 0;
    let mut out = Vec::new();
    for c in text.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = B64.iter().position(|x| *x == c).ok_or_else(|| ObsidianError::GitHub("bad base64 from GitHub".into()))? as u32;
        bits = bits << 6 | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n & 0xff) as u8);
        }
    }
    String::from_utf8(out).map_err(|_| ObsidianError::GitHub("mods.json is not text".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_names_stay_in_the_path() {
        assert_eq!(repo_name("https://github.com/Alomare/HD1-BetterHUD").as_deref(), Some("Alomare/HD1-BetterHUD"));
        assert_eq!(repo_name("github.com/a/b.git/").as_deref(), Some("a/b"));
        assert_eq!(repo_name("a/b").as_deref(), Some("a/b"));
        for bad in ["a", "a/b/c", "../b", "a/..", "a/b c", "a/b?x=1", "", "https://gitlab.com/a/b"] {
            assert_eq!(repo_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn base64_goes_both_ways() {
        for s in ["", "a", "ab", "abc", "abcd", "{\n \"mods\": []\n}\n", "Ünïcode"] {
            assert_eq!(decode_b64(&encode_b64(s.as_bytes())).unwrap(), s);
        }
        assert_eq!(encode_b64(b"Man"), "TWFu");
        assert_eq!(decode_b64("TWE=\n").unwrap(), "Ma");
    }

    #[test]
    fn a_submission_adds_one_line_to_mods() {
        let text = "{\n  \"verified\": [\"Alomare\"],\n  \"kit\": [\"Alomare/ObsidianAPI\"],\n  \"mods\": [\"Alomare/HD1-BetterHUD\"]\n}\n";
        let out = add_to_mods_json(text, "someone/Cool-Mod").unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mods"], json!(["Alomare/HD1-BetterHUD", "someone/Cool-Mod"]));
        assert_eq!(v["verified"], json!(["Alomare"]), "nothing else changes");
        assert_eq!(out, text.replace("\"Alomare/HD1-BetterHUD\"]", "\"Alomare/HD1-BetterHUD\", \"someone/Cool-Mod\"]"), "key order and layout kept");
        let tall = "{\n  \"verified\": [\n    \"Alomare\"\n  ],\n  \"mods\": [\n    \"a/b\"\n  ]\n}\n";
        assert_eq!(add_to_mods_json(tall, "c/d").unwrap(), "{\n  \"verified\": [\n    \"Alomare\"\n  ],\n  \"mods\": [\n    \"a/b\",\n    \"c/d\"\n  ]\n}\n");
        assert_eq!(add_to_mods_json("{\"mods\": []}", "c/d").unwrap(), "{\"mods\": [\"c/d\"]}");
        assert!(matches!(add_to_mods_json(text, "alomare/hd1-betterhud"), Err(ObsidianError::GitHub(_))));
        assert!(matches!(add_to_mods_json(text, "Alomare/ObsidianAPI"), Err(ObsidianError::GitHub(_))));
    }

    #[test]
    fn reports_keep_on_disk_and_forget_one() {
        let t = tempfile::tempdir().unwrap();
        assert!(reports(t.path()).is_empty());
        let r = |n| Report {
            repo: "a/b".into(), number: n, title: "T".into(), mod_id: "a.b".into(), mod_name: "B".into(), url: String::new(),
            created: String::new(), state: "open".into(), comments: 2, seen: 1, updated: String::new(),
        };
        save_reports(t.path(), &[r(1), r(2)]).unwrap();
        assert_eq!(reports(t.path()).len(), 2);
        assert_eq!(forget_report(t.path(), "a/b", 1).unwrap(), vec![r(2)]);
    }

    #[test]
    fn the_account_file_never_reaches_the_window_with_the_token() {
        let t = tempfile::tempdir().unwrap();
        assert!(load(t.path()).is_none());
        let s = Saved { token: "gho_secret".into(), login: "me".into(), avatar: None, scopes: vec!["repo".into()], sync_gist: None };
        save(t.path(), &s).unwrap();
        let a = Account::from(&saved(t.path()).unwrap());
        assert!(!serde_json::to_string(&a).unwrap().contains("gho_secret"));
        assert!(has_scope(t.path(), "public_repo"), "repo covers public_repo");
        assert!(!has_scope(t.path(), "gist"));
        // sign_out touches the shared token slot; the file is what this test checks.
        std::fs::remove_file(path(t.path())).unwrap();
        assert!(saved(t.path()).is_none());
    }
}
