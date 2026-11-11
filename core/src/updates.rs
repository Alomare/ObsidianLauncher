//! Mod updates: a mod whose `source` is a GitHub repository updates from that repository's latest
//! release. Nothing here runs unless the player asks, and a release can only replace the mod it
//! belongs to, with a higher version.
//!
//! GitHub's API takes 60 calls an hour from one internet address without a login, and Obsidian has
//! none, so a check spends none of them: it reads where github.com's own "latest release" page
//! points. The API is asked only for what the page can't tell (the files of a release, to install
//! it, and the changelogs), and when the page answers in a way this doesn't know.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::library::{InstallOutcome, Library};
use crate::manifest::{Manifest, load_manifest};
use crate::{ObsidianError, Result};

/// The largest file Obsidian downloads, so a bad release can't fill the disk.
const MAX_DOWNLOAD: u64 = 1 << 30;

#[derive(Debug, Clone, PartialEq)]
pub struct Repo {
    owner: String,
    name: String,
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// The repository in a `source` address: `https://github.com/<owner>/<repo>`, ignoring what follows.
/// Anything else, a plain `http` address included, is not one Obsidian updates from.
pub fn github_repo(source: &str) -> Option<Repo> {
    let rest = source.trim().strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let name = name.strip_suffix(".git").unwrap_or(name);
    let fits = |s: &str| !s.is_empty() && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
    (fits(owner) && fits(name)).then(|| Repo { owner: owner.to_string(), name: name.to_string() })
}

/// The repository a mod updates from.
pub fn source_of(m: &Manifest) -> Option<Repo> {
    github_repo(m.extra.get("source")?.as_str()?)
}

#[derive(Debug, Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct RawRelease {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct RawNotes {
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawListed {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// A published release as the changelog shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ListedRelease {
    pub version: String,
    pub date: Option<String>,
    pub notes: String,
    pub url: String,
}

#[derive(Debug)]
pub struct Release {
    pub version: Version,
    assets: Vec<Asset>,
}

pub struct GitHub {
    api: String,
    /// github.com itself, for the latest-release page.
    web: String,
    agent: ureq::Agent,
    /// The same, but it hands back a redirect instead of following it.
    pages: ureq::Agent,
}

impl Default for GitHub {
    /// The real GitHub, or `OBSIDIAN_GITHUB_API` (tests), which then stands in for github.com too.
    fn default() -> Self {
        match std::env::var("OBSIDIAN_GITHUB_API") {
            Ok(base) => GitHub::at(base),
            Err(_) => GitHub::with("https://api.github.com", "https://github.com"),
        }
    }
}

fn network(e: impl fmt::Display) -> ObsidianError {
    ObsidianError::UpdateNetwork(e.to_string())
}

/// The version a tag names, with or without the `v`.
fn version_of(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix(['v', 'V']).unwrap_or(tag)).ok()
}

/// `%2B` and the like back into the characters they stand for.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 3 <= b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether a refused API call was the rate limit: the hourly one (no calls left), or the one GitHub
/// keeps for bursts, which asks to wait (`retry-after`) or says so in its text.
fn rate_limited(status: u16, remaining: Option<&str>, retry_after: bool, text: &str) -> bool {
    status == 429 || remaining == Some("0") || retry_after || text.to_lowercase().contains("rate limit")
}

impl GitHub {
    /// One server for the API and the pages alike: a stand-in for tests.
    pub fn at(base: impl Into<String>) -> Self {
        let base = base.into();
        GitHub::with(base.clone(), base)
    }

    pub fn with(api: impl Into<String>, web: impl Into<String>) -> Self {
        let agent = |redirects: u32| -> ureq::Agent {
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .max_redirects(redirects)
                .user_agent(concat!("Obsidian/", env!("CARGO_PKG_VERSION")))
                .timeout_connect(Some(Duration::from_secs(15)))
                .timeout_recv_response(Some(Duration::from_secs(30)))
                .timeout_recv_body(Some(Duration::from_secs(600)))
                .build()
                .into()
        };
        GitHub { api: api.into(), web: web.into(), agent: agent(10), pages: agent(0) }
    }

    /// The newest release of `repo`, with its files. GitHub leaves out drafts and prereleases.
    /// This is an API call.
    pub fn latest(&self, repo: &Repo) -> Result<Release> {
        let raw: RawRelease = serde_json::from_str(&self.release_json(repo, "/latest")?)?;
        let version = version_of(&raw.tag_name).ok_or_else(|| ObsidianError::NoRelease(repo.to_string()))?;
        Ok(Release { version, assets: raw.assets })
    }

    /// The version of the newest release, which is all a check needs. It comes from the
    /// latest-release page, which costs no API call; the API is asked only when the page answers in
    /// a way this doesn't know, and only while `api` allows it.
    fn latest_version(&self, repo: &Repo, api: bool) -> Result<Version> {
        match self.latest_tag_from_page(repo)? {
            Some(tag) => version_of(&tag).ok_or_else(|| ObsidianError::NoRelease(repo.to_string())),
            None if api => Ok(self.latest(repo)?.version),
            None => Err(ObsidianError::UpdateLimited),
        }
    }

    /// The version of the newest release of `repo`, from the page first and the API when needed.
    pub fn newest_version(&self, repo: &Repo) -> Result<Version> {
        self.latest_version(repo, true)
    }

    /// The tag that github.com's latest-release page sends a browser to (`/releases/tag/<tag>`).
    /// A repository without a release sends it to `/releases`, and a missing one answers 404. A
    /// moved repository sends it to the same page under the new name, which is followed for a few
    /// hops on the same site. `None`: an answer this doesn't know, for the API to settle.
    fn latest_tag_from_page(&self, repo: &Repo) -> Result<Option<String>> {
        let mut url = format!("{}/{repo}/releases/latest", self.web);
        for _ in 0..3 {
            let res = self.pages.get(&url).call().map_err(network)?;
            let status = res.status().as_u16();
            if status == 404 {
                return Err(ObsidianError::NoRelease(repo.to_string()));
            }
            if !(300..400).contains(&status) {
                return Ok(None);
            }
            let Some(to) = res.headers().get("location").and_then(|v| v.to_str().ok()) else { return Ok(None) };
            let to = to.split(['?', '#']).next().unwrap_or_default();
            if let Some((_, tag)) = to.split_once("/releases/tag/") {
                return Ok(Some(percent_decode(tag)));
            }
            if to.ends_with("/releases") {
                return Err(ObsidianError::NoRelease(repo.to_string()));
            }
            if !to.ends_with("/releases/latest") {
                return Ok(None);
            }
            url = if to.starts_with('/') {
                format!("{}{to}", self.web)
            } else if to.starts_with(&format!("{}/", self.web)) {
                to.to_string()
            } else {
                return Ok(None);
            };
        }
        Ok(None)
    }

    /// The notes of one release as GitHub has them now, so an edit there counts.
    pub fn notes(&self, repo: &Repo, tag: &str) -> Result<Option<String>> {
        let raw: RawNotes = serde_json::from_str(&self.release_json(repo, &format!("/tags/{tag}"))?)?;
        Ok(raw.body.filter(|b| !b.trim().is_empty()))
    }

    /// The published releases of `repo`, newest first. A tag that isn't a version is left out.
    pub fn releases(&self, repo: &Repo) -> Result<Vec<ListedRelease>> {
        Ok(self.releases_page(repo, 1, 50)?.0)
    }

    /// One page of `releases`, counted from 1, and whether GitHub may have more after it.
    pub fn releases_page(&self, repo: &Repo, page: u32, per_page: u32) -> Result<(Vec<ListedRelease>, bool)> {
        let per_page = per_page.clamp(1, 100);
        let raw: Vec<RawListed> = serde_json::from_str(&self.release_json(repo, &format!("?per_page={per_page}&page={}", page.max(1)))?)?;
        let more = raw.len() as u32 == per_page;
        let list = raw
            .into_iter()
            .filter(|r| !r.draft && !r.prerelease)
            .filter_map(|r| {
                let version = r.tag_name.strip_prefix(['v', 'V']).unwrap_or(&r.tag_name);
                Version::parse(version).ok()?;
                Some(ListedRelease { version: version.to_string(), date: r.published_at, notes: r.body.unwrap_or_default(), url: r.html_url })
            })
            .collect();
        Ok((list, more))
    }

    fn release_json(&self, repo: &Repo, which: &str) -> Result<String> {
        let url = format!("{}/repos/{repo}/releases{which}", self.api);
        let mut res = self.agent.get(&url).header("Accept", "application/vnd.github+json").call().map_err(network)?;
        match res.status().as_u16() {
            200 => {}
            404 => return Err(ObsidianError::NoRelease(repo.to_string())),
            code @ (403 | 429) => {
                let remaining = res.headers().get("x-ratelimit-remaining").and_then(|v| v.to_str().ok()).map(str::to_string);
                let retry_after = res.headers().contains_key("retry-after");
                let text = res.body_mut().read_to_string().unwrap_or_default();
                if rate_limited(code, remaining.as_deref(), retry_after, &text) {
                    return Err(ObsidianError::UpdateLimited);
                }
                return Err(network(format!("GitHub answered {code}")));
            }
            code => return Err(network(format!("GitHub answered {code}"))),
        }
        res.body_mut().read_to_string().map_err(network)
    }

    /// Downloads the ZIP of `repo`'s newest release whose name starts with `prefix` (any case) into
    /// `to`, and gives the release's version. For a program Obsidian installs from a release of its
    /// own, such as the Commander package; mods go through `stage`.
    pub fn download_latest_zip(&self, repo: &Repo, prefix: &str, to: &Path) -> Result<Version> {
        let release = self.latest(repo)?;
        let prefix = prefix.to_lowercase();
        let asset = release
            .assets
            .iter()
            .find(|a| {
                let n = a.name.to_lowercase();
                n.starts_with(&prefix) && n.ends_with(".zip")
            })
            .ok_or_else(|| ObsidianError::NoUpdateAsset(repo.to_string()))?;
        self.download(&asset.browser_download_url, to)?;
        Ok(release.version)
    }

    fn download(&self, url: &str, to: &Path) -> Result<()> {
        let mut res = self.agent.get(url).call().map_err(network)?;
        if !res.status().is_success() {
            return Err(network(format!("GitHub answered {}", res.status().as_u16())));
        }
        let mut body = res.body_mut().with_config().limit(MAX_DOWNLOAD).reader();
        std::io::copy(&mut body, &mut std::fs::File::create(to)?).map_err(network)?;
        Ok(())
    }
}

/// What a check found for one mod.
pub struct Checked {
    pub id: String,
    /// The newer version, or `None` when the mod is up to date.
    pub result: Result<Option<Version>>,
}

/// Asks GitHub about every mod that names a repository. One failure doesn't stop the rest. Once
/// the API's limit is reached it isn't asked again in the same check, since GitHub counts the calls
/// made after it too, but the pages still are.
pub fn check(github: &GitHub, mods: &[Manifest]) -> Vec<Checked> {
    latest_of(github, mods, &mut BTreeMap::new(), None, 0)
}

/// The file in the data folder that keeps what the last checks found.
pub const MEMORY_FILE: &str = "update-checks.json";
/// How long the check at startup trusts an answer instead of asking again: restarting Obsidian a
/// few times in an afternoon asks GitHub once.
pub const TRUST_SECS: u64 = 6 * 3600;
/// Answers older than this leave the file, so mods removed long ago don't stay in it.
const FORGET_SECS: u64 = 30 * 24 * 3600;

/// One mod's last answer: the newest version its repository had, and when that was asked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Seen {
    source: String,
    latest: String,
    at: u64,
}

/// Seconds since 1970, the clock the memory file uses.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `check`, keeping each answer in `file`. With `trust`, an answer for the same repository that is
/// younger than that many seconds stands in for asking. A failed check keeps nothing, so it asks
/// again next time, and a file that can't be read or written only means asking.
pub fn check_remembering(github: &GitHub, mods: &[Manifest], file: &Path, trust: Option<u64>, now: u64) -> Vec<Checked> {
    let mut seen: BTreeMap<String, Seen> = std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let before = seen.clone();
    let checked = latest_of(github, mods, &mut seen, trust, now);
    seen.retain(|_, s| s.at > now.saturating_sub(FORGET_SECS));
    if seen != before
        && let Ok(text) = serde_json::to_vec_pretty(&seen)
    {
        // Not keeping an answer only costs a question next time.
        let _ = crate::paths::write_atomic(file, &text);
    }
    checked
}

fn latest_of(github: &GitHub, mods: &[Manifest], seen: &mut BTreeMap<String, Seen>, trust: Option<u64>, now: u64) -> Vec<Checked> {
    let mut api = true;
    mods.iter()
        .filter_map(|m| {
            let repo = source_of(m)?;
            let source = repo.to_string();
            let kept = seen
                .get(&m.id)
                .filter(|s| trust.is_some_and(|t| s.source == source && s.at <= now && now - s.at < t))
                .and_then(|s| Version::parse(&s.latest).ok());
            let latest = match kept {
                Some(v) => Ok(v),
                None => {
                    let asked = github.latest_version(&repo, api);
                    match &asked {
                        Ok(v) => {
                            seen.insert(m.id.clone(), Seen { source, latest: v.to_string(), at: now });
                        }
                        Err(ObsidianError::UpdateLimited) => api = false,
                        Err(_) => {}
                    }
                    asked
                }
            };
            Some(Checked { id: m.id.clone(), result: latest.map(|v| (v > m.version).then_some(v)) })
        })
        .collect()
}

/// The `.zip` of a release: the only one, or the only one with the mod id in its name.
fn pick_zip<'a>(assets: &'a [Asset], id: &str) -> Option<&'a Asset> {
    let zips: Vec<&Asset> = assets.iter().filter(|a| a.name.to_lowercase().ends_with(".zip")).collect();
    if let [one] = zips.as_slice() {
        return Some(one);
    }
    match zips.iter().filter(|a| a.name.to_lowercase().contains(id)).collect::<Vec<_>>().as_slice() {
        [one] => Some(one),
        _ => None,
    }
}

/// A checked update, unpacked and ready to go into the library.
pub struct Staged {
    _dir: tempfile::TempDir,
    root: PathBuf,
    pub manifest: Manifest,
}

impl Staged {
    pub fn install(&self, lib: &Library) -> Result<InstallOutcome> {
        lib.install(&self.root)
    }
}

/// Downloads the latest release of a mod and checks that it is this mod, in a higher version. This
/// is the slow part and touches nothing in the library, so it doesn't need to hold anything up.
pub fn stage(github: &GitHub, current: &Manifest) -> Result<Staged> {
    let mismatch = |found: String| ObsidianError::UpdateMismatch { wanted: current.id.clone(), found };
    let repo = source_of(current).ok_or_else(|| ObsidianError::NoUpdateSource(current.id.clone()))?;
    let release = github.latest(&repo)?;
    if release.version <= current.version {
        return Err(mismatch(format!("{} {}", current.id, release.version)));
    }
    let staged = unpack(github, &repo, &release, &current.id)?;
    if staged.manifest.id != current.id || staged.manifest.version <= current.version {
        return Err(mismatch(format!("{} {}", staged.manifest.id, staged.manifest.version)));
    }
    Ok(staged)
}

/// Downloads the latest release of a mod that isn't installed yet, from `repo`, and checks that it
/// is the mod `id`. Like `stage`, it touches nothing in the library.
pub fn stage_new(github: &GitHub, repo: &Repo, id: &str) -> Result<Staged> {
    let release = github.latest(repo)?;
    let staged = unpack(github, repo, &release, id)?;
    if staged.manifest.id != id {
        return Err(ObsidianError::UpdateMismatch { wanted: id.to_string(), found: format!("{} {}", staged.manifest.id, staged.manifest.version) });
    }
    Ok(staged)
}

/// The mod ZIP of a release, downloaded and unpacked into a temporary folder.
fn unpack(github: &GitHub, repo: &Repo, release: &Release, id: &str) -> Result<Staged> {
    let asset = pick_zip(&release.assets, id).ok_or_else(|| ObsidianError::NoUpdateAsset(repo.to_string()))?;
    let dir = tempfile::tempdir()?;
    let (zip, unpacked) = (dir.path().join("update.zip"), dir.path().join("mod"));
    github.download(&asset.browser_download_url, &zip)?;
    crate::import::unzip(&zip, &unpacked)?;
    let root = crate::import::mod_folder(&unpacked)?.ok_or_else(|| ObsidianError::NoUpdateAsset(repo.to_string()))?;
    let manifest = load_manifest(&root)?;
    Ok(Staged { _dir: dir, root, manifest })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::spaced_tempdir;
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex};
    use zip::write::SimpleFileOptions;

    struct Route {
        path: String,
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    /// A stand-in for GitHub: answers each path with a status, headers and a body, and 404 for the
    /// rest, and notes every path asked for.
    #[derive(Clone, Default)]
    struct Fake {
        base: String,
        routes: Arc<Mutex<Vec<Route>>>,
        hits: Arc<Mutex<Vec<String>>>,
    }

    impl Fake {
        fn serve(&self, path: &str, status: u16, headers: &[(&str, &str)], body: impl Into<Vec<u8>>) {
            let headers = headers.iter().map(|(k, v)| (k.to_string(), v.replace("{base}", &self.base))).collect();
            self.routes.lock().unwrap().push(Route { path: path.into(), status, headers, body: body.into() });
        }
        fn ok(&self, path: &str, body: impl Into<Vec<u8>>) {
            self.serve(path, 200, &[], body);
        }
        /// The latest-release page of me/mod sending the browser to `to`.
        fn page(&self, status: u16, to: &str) {
            self.serve("/me/mod/releases/latest", status, &[("Location", to)], Vec::new());
        }
        fn hits(&self) -> Vec<String> {
            self.hits.lock().unwrap().clone()
        }
        fn api_calls(&self) -> usize {
            self.hits().iter().filter(|p| p.starts_with("/repos/")).count()
        }
        fn github(&self) -> GitHub {
            GitHub::at(self.base.clone())
        }
    }

    fn server() -> Fake {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let fake = Fake { base: format!("http://{}", listener.local_addr().unwrap()), ..Fake::default() };
        let served = fake.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let path = first.split(' ').nth(1).unwrap_or("").to_string();
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 2 {
                    line.clear();
                }
                served.hits.lock().unwrap().push(path.clone());
                let routes = served.routes.lock().unwrap();
                let (status, headers, body) = routes.iter().find(|r| r.path == path).map(|r| (r.status, r.headers.clone(), r.body.clone())).unwrap_or((404, Vec::new(), Vec::new()));
                drop(routes);
                let extra: String = headers.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
                write!(stream, "HTTP/1.1 {status} X\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        fake
    }

    /// GitHub with one release of me/mod, on the latest-release page and in the API. Each asset is
    /// served at `/dl/<name>`.
    fn fake_github(tag: &str, assets: Vec<(&str, Vec<u8>)>) -> Fake {
        let fake = server();
        let base = fake.base.clone();
        let list: Vec<String> = assets.iter().map(|(n, _)| format!(r#"{{"name":"{n}","browser_download_url":"{base}/dl/{n}"}}"#)).collect();
        fake.ok("/repos/me/mod/releases/latest", format!(r#"{{"tag_name":"{tag}","assets":[{}]}}"#, list.join(",")));
        fake.page(302, &format!("{base}/me/mod/releases/tag/{tag}"));
        for (n, b) in assets {
            fake.ok(&format!("/dl/{n}"), b);
        }
        fake
    }

    fn github(tag: &str, assets: Vec<(&str, Vec<u8>)>) -> GitHub {
        fake_github(tag, assets).github()
    }

    fn manifest(id: &str, version: &str) -> String {
        format!(r#"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}},"source":"https://github.com/me/mod"}}"#)
    }

    fn installed(id: &str, version: &str) -> Manifest {
        serde_json::from_str(&manifest(id, version)).unwrap()
    }

    fn zipped(folder: &str, json: &str) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file(format!("{folder}obsidian.json"), SimpleFileOptions::default()).unwrap();
        z.write_all(json.as_bytes()).unwrap();
        z.finish().unwrap().into_inner()
    }

    fn found(g: &GitHub, version: &str) -> Option<Version> {
        check(g, &[installed("a.mod", version)]).pop().unwrap().result.unwrap()
    }

    #[test]
    fn only_github_addresses_count() {
        let repo = |s: &str| github_repo(s).map(|r| r.to_string());
        assert_eq!(repo("https://github.com/me/mod").as_deref(), Some("me/mod"));
        assert_eq!(repo(" https://github.com/me/mod.git ").as_deref(), Some("me/mod"));
        assert_eq!(repo("https://github.com/me/mod/releases/tag/v1").as_deref(), Some("me/mod"));
        for bad in ["http://github.com/me/mod", "https://gitlab.com/me/mod", "https://github.com/me", "https://github.com/../mod", "https://github.com/me/a b", "me/mod"] {
            assert_eq!(repo(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_higher_release_is_found_with_or_without_the_v() {
        assert_eq!(found(&github("v1.2.0", vec![]), "1.1.0"), Some(Version::new(1, 2, 0)));
        assert_eq!(found(&github("1.2.0", vec![]), "1.1.0"), Some(Version::new(1, 2, 0)));
    }

    #[test]
    fn the_same_or_a_lower_release_is_no_update() {
        assert_eq!(found(&github("v1.2.0", vec![]), "1.2.0"), None);
        assert_eq!(found(&github("v1.2.0", vec![]), "2.0.0"), None);
    }

    #[test]
    fn mods_without_a_repository_are_left_out_of_a_check() {
        let plain: Manifest = serde_json::from_str(r#"{"obsidian":1,"id":"b.mod","version":"1.0.0","name":{"en":"B"}}"#).unwrap();
        assert!(check(&github("v1.0.0", vec![]), &[plain]).is_empty());
    }

    #[test]
    fn the_changelog_lists_published_versions_only() {
        let fake = server();
        let list = br#"[
            {"tag_name":"v1.0.2","html_url":"u2","published_at":"2026-10-31T00:00:00Z","body":"Two"},
            {"tag_name":"v1.0.3","html_url":"u3","draft":true},
            {"tag_name":"v1.1.0-beta.1","html_url":"u4","prerelease":true},
            {"tag_name":"nightly","html_url":"u5"},
            {"tag_name":"1.0.1","html_url":"u1","body":null}
        ]"#;
        fake.ok("/repos/me/mod/releases?per_page=50&page=1", list.to_vec());
        let github = fake.github();
        let found = github.releases(&github_repo("https://github.com/me/mod").unwrap()).unwrap();
        let versions: Vec<&str> = found.iter().map(|r| r.version.as_str()).collect();
        assert_eq!(versions, ["1.0.2", "1.0.1"]);
        assert_eq!(found[0].notes, "Two");
        assert_eq!(found[1].notes, "");
    }

    #[test]
    fn a_full_page_of_releases_says_more_may_follow() {
        let fake = server();
        let page = |a: &str, b: &str| format!(r#"[{{"tag_name":"{a}","html_url":"u"}},{{"tag_name":"{b}","html_url":"u"}}]"#).into_bytes();
        fake.ok("/repos/me/mod/releases?per_page=2&page=1", page("v1.3.0", "v1.2.0"));
        fake.ok("/repos/me/mod/releases?per_page=2&page=2", br#"[{"tag_name":"v1.1.0","html_url":"u"}]"#.to_vec());
        let github = fake.github();
        let repo = github_repo("https://github.com/me/mod").unwrap();
        let (first, more) = github.releases_page(&repo, 1, 2).unwrap();
        assert_eq!((first.len(), more), (2, true));
        let (second, more) = github.releases_page(&repo, 2, 2).unwrap();
        assert_eq!((second[0].version.as_str(), more), ("1.1.0", false));
    }

    #[test]
    fn release_notes_read_as_github_has_them_now() {
        let fake = server();
        fake.ok("/repos/me/mod/releases/tags/v1.0.1", br#"{"body":"Edited on GitHub"}"#.to_vec());
        fake.ok("/repos/me/mod/releases/tags/v1.0.2", br#"{"body":"  "}"#.to_vec());
        fake.ok("/repos/me/mod/releases/tags/v1.0.3", br#"{"body":null}"#.to_vec());
        let github = fake.github();
        let repo = github_repo("https://github.com/me/mod").unwrap();
        assert_eq!(github.notes(&repo, "v1.0.1").unwrap().as_deref(), Some("Edited on GitHub"));
        assert_eq!(github.notes(&repo, "v1.0.2").unwrap(), None, "blank text counts as none");
        assert_eq!(github.notes(&repo, "v1.0.3").unwrap(), None);
        assert!(matches!(github.notes(&repo, "v9.9.9"), Err(ObsidianError::NoRelease(_))));
    }

    #[test]
    fn a_failed_check_names_why_and_the_next_mod_still_runs() {
        let fake = server();
        // me/mod: no repository (404); me/refused: a page this doesn't know, then an API refusal
        // that isn't the limit.
        fake.ok("/me/refused/releases/latest", "<html>");
        fake.serve("/repos/me/refused/releases/latest", 403, &[], Vec::new());
        let r = check(&fake.github(), &[installed("a.mod", "1.0.0"), sourced("b.mod", "1.0.0", "me/refused")]);
        assert!(matches!(r[0].result, Err(ObsidianError::NoRelease(_))), "a missing repository");
        assert!(matches!(&r[1].result, Err(ObsidianError::UpdateNetwork(m)) if m.contains("403")));
    }

    fn sourced(id: &str, version: &str, repo: &str) -> Manifest {
        let mut m = installed(id, version);
        m.extra.insert("source".into(), format!("https://github.com/{repo}").into());
        m
    }

    #[test]
    fn a_check_reads_the_latest_release_page_and_spends_no_api_call() {
        let fake = server();
        fake.page(302, "{base}/me/mod/releases/tag/v1.2.0");
        assert_eq!(found(&fake.github(), "1.1.0"), Some(Version::new(1, 2, 0)));
        assert_eq!(fake.hits(), ["/me/mod/releases/latest"]);
        assert_eq!(fake.api_calls(), 0);
    }

    #[test]
    fn a_page_on_another_path_or_with_an_encoded_tag_still_names_it() {
        let fake = server();
        fake.page(302, "/me/mod/releases/tag/v1.2.0%2Bbuild.5?from=latest");
        assert_eq!(found(&fake.github(), "1.1.0"), Some(Version::parse("1.2.0+build.5").unwrap()));
        assert_eq!(fake.api_calls(), 0);
    }

    #[test]
    fn a_repository_without_a_release_or_a_missing_one_is_no_release() {
        let none = server();
        none.page(302, "{base}/me/mod/releases");
        let missing = server();
        for fake in [none, missing] {
            let r = check(&fake.github(), &[installed("a.mod", "1.0.0")]).pop().unwrap().result;
            assert!(matches!(r, Err(ObsidianError::NoRelease(_))));
            assert_eq!(fake.api_calls(), 0);
        }
    }

    #[test]
    fn a_page_tag_that_is_no_version_is_no_release() {
        let fake = server();
        fake.page(302, "{base}/me/mod/releases/tag/nightly");
        let r = check(&fake.github(), &[installed("a.mod", "1.0.0")]).pop().unwrap().result;
        assert!(matches!(r, Err(ObsidianError::NoRelease(_))));
    }

    #[test]
    fn a_moved_repository_is_followed_to_its_new_name() {
        let fake = server();
        fake.page(301, "{base}/me/renamed/releases/latest");
        fake.serve("/me/renamed/releases/latest", 302, &[("Location", "{base}/me/renamed/releases/tag/v2.0.0")], Vec::new());
        assert_eq!(found(&fake.github(), "1.0.0"), Some(Version::new(2, 0, 0)));
        assert_eq!(fake.api_calls(), 0);
    }

    #[test]
    fn an_answer_the_page_route_doesnt_know_goes_to_the_api() {
        // A page that loads instead of sending on, one that throttles, a hop to another site, and
        // a loop of moves: the API settles each.
        for (status, to) in [(200, ""), (429, ""), (301, "https://elsewhere.example/me/mod/releases/latest"), (301, "{base}/me/mod/releases/latest")] {
            let fake = server();
            fake.page(status, to);
            fake.ok("/repos/me/mod/releases/latest", r#"{"tag_name":"v1.3.0","assets":[]}"#);
            assert_eq!(found(&fake.github(), "1.0.0"), Some(Version::new(1, 3, 0)), "{status} {to}");
            assert_eq!(fake.api_calls(), 1, "{status} {to}");
        }
    }

    /// A refused API call: status, headers, text.
    type Refusal<'a> = (u16, &'a [(&'a str, &'a str)], &'a str);

    #[test]
    fn the_api_limit_is_known_by_its_headers_or_its_text() {
        let cases: [Refusal; 4] = [
            (403, &[("x-ratelimit-remaining", "0")], ""),
            (429, &[], ""),
            (403, &[("retry-after", "60")], ""),
            (403, &[], r#"{"message":"You have exceeded a secondary rate limit."}"#),
        ];
        for (status, headers, text) in cases {
            let fake = server();
            fake.ok("/me/mod/releases/latest", "<html>");
            fake.serve("/repos/me/mod/releases/latest", status, headers, text);
            let r = check(&fake.github(), &[installed("a.mod", "1.0.0")]).pop().unwrap().result;
            assert!(matches!(r, Err(ObsidianError::UpdateLimited)), "{status} {headers:?} {text}");
        }
    }

    #[test]
    fn a_reached_api_limit_doesnt_stop_the_page_checks_of_the_others() {
        let fake = server();
        fake.ok("/me/mod/releases/latest", "<html>");
        fake.serve("/repos/me/mod/releases/latest", 403, &[("x-ratelimit-remaining", "0")], Vec::new());
        fake.serve("/me/paged/releases/latest", 302, &[("Location", "{base}/me/paged/releases/tag/v2.0.0")], Vec::new());
        fake.ok("/me/odd/releases/latest", "<html>");
        let mods = [installed("a.mod", "1.0.0"), sourced("b.mod", "1.0.0", "me/paged"), sourced("c.mod", "1.0.0", "me/odd")];
        let r = check(&fake.github(), &mods);
        assert!(matches!(r[0].result, Err(ObsidianError::UpdateLimited)));
        assert_eq!(r[1].result.as_ref().unwrap(), &Some(Version::new(2, 0, 0)), "the page still answers");
        assert!(matches!(r[2].result, Err(ObsidianError::UpdateLimited)), "the API isn't asked again");
        assert_eq!(fake.api_calls(), 1);
    }

    const HOUR: u64 = 3600;
    const T0: u64 = 1_800_000_000;

    fn remembering(fake: &Fake, file: &Path, mods: &[Manifest], trust: Option<u64>, now: u64) -> Vec<Option<Version>> {
        check_remembering(&fake.github(), mods, file, trust, now).into_iter().map(|c| c.result.unwrap()).collect()
    }

    #[test]
    fn the_startup_check_trusts_an_answer_from_the_last_hours() {
        let t = spaced_tempdir();
        let file = t.path().join(MEMORY_FILE);
        let fake = fake_github("v1.2.0", vec![]);
        assert_eq!(remembering(&fake, &file, &[installed("a.mod", "1.0.0")], None, T0), [Some(Version::new(1, 2, 0))]);
        assert_eq!(fake.hits().len(), 1);
        assert!(file.exists());

        let later = T0 + TRUST_SECS - HOUR;
        assert_eq!(remembering(&fake, &file, &[installed("a.mod", "1.0.0")], Some(TRUST_SECS), later), [Some(Version::new(1, 2, 0))]);
        assert_eq!(remembering(&fake, &file, &[installed("a.mod", "1.2.0")], Some(TRUST_SECS), later), [None], "the installed version counts as it is now");
        assert_eq!(fake.hits().len(), 1, "nothing asked GitHub again");
    }

    #[test]
    fn a_check_the_player_starts_always_asks() {
        let t = spaced_tempdir();
        let file = t.path().join(MEMORY_FILE);
        let fake = fake_github("v1.2.0", vec![]);
        remembering(&fake, &file, &[installed("a.mod", "1.0.0")], None, T0);
        remembering(&fake, &file, &[installed("a.mod", "1.0.0")], None, T0 + 60);
        assert_eq!(fake.hits().len(), 2);
    }

    #[test]
    fn an_old_answer_a_new_source_or_a_clock_set_back_asks_again() {
        let t = spaced_tempdir();
        let file = t.path().join(MEMORY_FILE);
        let fake = fake_github("v1.2.0", vec![]);
        fake.serve("/me/other/releases/latest", 302, &[("Location", "{base}/me/other/releases/tag/v3.0.0")], Vec::new());
        remembering(&fake, &file, &[installed("a.mod", "1.0.0")], None, T0);
        remembering(&fake, &file, &[installed("a.mod", "1.0.0")], Some(TRUST_SECS), T0 + TRUST_SECS);
        assert_eq!(fake.hits().len(), 2, "an answer as old as the trust asks again");
        let moved = remembering(&fake, &file, &[sourced("a.mod", "1.0.0", "me/other")], Some(TRUST_SECS), T0 + TRUST_SECS + 60);
        assert_eq!(moved, [Some(Version::new(3, 0, 0))], "a mod that names another repository");
        assert_eq!(fake.hits().len(), 3);
        remembering(&fake, &file, &[sourced("a.mod", "1.0.0", "me/other")], Some(TRUST_SECS), T0);
        assert_eq!(fake.hits().len(), 4, "an answer from the future");
    }

    #[test]
    fn a_failed_check_is_not_remembered() {
        let t = spaced_tempdir();
        let file = t.path().join(MEMORY_FILE);
        let fake = server();
        for _ in 0..2 {
            let r = check_remembering(&fake.github(), &[installed("a.mod", "1.0.0")], &file, Some(TRUST_SECS), T0).pop().unwrap().result;
            assert!(matches!(r, Err(ObsidianError::NoRelease(_))));
        }
        assert_eq!(fake.hits().len(), 2);
        assert!(!file.exists(), "nothing worth keeping");
    }

    #[test]
    fn a_broken_or_unwritable_memory_only_means_asking() {
        let t = spaced_tempdir();
        let file = t.path().join(MEMORY_FILE);
        std::fs::write(&file, b"{ torn").unwrap();
        let fake = fake_github("v1.2.0", vec![]);
        assert_eq!(remembering(&fake, &file, &[installed("a.mod", "1.0.0")], Some(TRUST_SECS), T0), [Some(Version::new(1, 2, 0))]);
        assert!(serde_json::from_slice::<serde_json::Value>(&std::fs::read(&file).unwrap()).is_ok(), "written anew");

        let nowhere = t.path().join("no such folder").join(MEMORY_FILE);
        assert_eq!(remembering(&fake, &nowhere, &[installed("a.mod", "1.0.0")], Some(TRUST_SECS), T0), [Some(Version::new(1, 2, 0))]);
    }

    #[test]
    fn answers_from_a_month_ago_leave_the_file() {
        let t = spaced_tempdir();
        let file = t.path().join(MEMORY_FILE);
        let fake = fake_github("v1.2.0", vec![]);
        remembering(&fake, &file, &[installed("gone.mod", "1.0.0")], None, T0);
        remembering(&fake, &file, &[installed("a.mod", "1.0.0")], None, T0 + FORGET_SECS + HOUR);
        let kept: BTreeMap<String, Seen> = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(kept.keys().collect::<Vec<_>>(), ["a.mod"]);
    }

    #[test]
    fn percent_decoding_leaves_plain_text_and_odd_escapes_alone() {
        assert_eq!(percent_decode("v1.2.0"), "v1.2.0");
        assert_eq!(percent_decode("v1.2.0%2Bbuild"), "v1.2.0+build");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%zzb"), "a%zzb");
    }

    /// The real GitHub, run by hand: `cargo test -p obsidian-core real_github -- --ignored --nocapture`.
    /// The page checks must agree with the API and leave the API's count where it was.
    #[test]
    #[ignore = "asks the real GitHub"]
    fn real_github_pages_agree_with_the_api_and_spend_none_of_it() {
        let quota = || -> u64 {
            let agent: ureq::Agent = ureq::Agent::config_builder().user_agent("Obsidian-test").build().into();
            let text = agent.get("https://api.github.com/rate_limit").call().unwrap().body_mut().read_to_string().unwrap();
            serde_json::from_str::<serde_json::Value>(&text).unwrap()["resources"]["core"]["remaining"].as_u64().unwrap()
        };
        let g = GitHub::with("https://api.github.com", "https://github.com");
        let repo = |s: &str| github_repo(&format!("https://github.com/{s}")).unwrap();
        let before = quota();
        let mut tags = Vec::new();
        for _ in 0..5 {
            for r in ["Alomare/HD1-PhotoMode", "Alomare/HD1-BetterHUD", "Alomare/HelldiversArcade"] {
                tags.push((r, g.latest_tag_from_page(&repo(r)).unwrap().expect("a tag")));
            }
        }
        assert!(matches!(g.latest_tag_from_page(&repo("octocat/Hello-World")), Err(ObsidianError::NoRelease(_))), "no releases");
        assert!(matches!(g.latest_tag_from_page(&repo("Alomare/no-such-repository-here")), Err(ObsidianError::NoRelease(_))), "missing");
        let moved = g.latest_tag_from_page(&repo("rust-lang-nursery/futures-rs")).unwrap();
        assert!(moved.is_some(), "a moved repository");
        let after = quota();
        println!("API calls left: {before} before, {after} after {} page checks; tags {tags:?}, moved {moved:?}", tags.len() + 3);
        assert!(after >= before, "the page checks spent API calls");
        let api = g.latest(&repo("Alomare/HD1-PhotoMode")).unwrap().version;
        assert_eq!(version_of(&tags[0].1), Some(api), "the page and the API name the same release");
    }

    #[test]
    fn a_tag_that_is_no_version_is_no_release() {
        let r = github("latest", vec![]).latest(&github_repo("https://github.com/me/mod").unwrap());
        assert!(matches!(r, Err(ObsidianError::NoRelease(_))));
    }

    #[test]
    fn an_update_replaces_the_mod_in_the_library() {
        let t = spaced_tempdir();
        let lib = Library::open(t.path()).unwrap();
        let old = t.path().join("old");
        crate::testutil::put(&old, "obsidian.json", manifest("a.mod", "1.0.0").as_bytes());
        lib.install(&old).unwrap();

        let g = github("v1.1.0", vec![("mod.zip", zipped("mod-1.1.0/", &manifest("a.mod", "1.1.0")))]);
        let staged = stage(&g, &load_manifest(&lib.mod_dir("a.mod")).unwrap()).unwrap();
        assert!(load_manifest(&lib.mod_dir("a.mod")).unwrap().version == Version::new(1, 0, 0), "staging leaves the library alone");

        let outcome = staged.install(&lib).unwrap();
        assert_eq!(outcome, InstallOutcome::Replaced { old_version: Version::new(1, 0, 0) });
        assert_eq!(load_manifest(&lib.mod_dir("a.mod")).unwrap().version, Version::new(1, 1, 0));
    }

    #[test]
    fn a_program_zip_comes_from_the_latest_release_by_name() {
        let t = spaced_tempdir();
        let to = t.path().join("p.zip");
        let g = github("v0.2.0", vec![("notes.txt", b"x".to_vec()), ("Obsidian-Commander-0.2.0.zip", b"PK-zip".to_vec())]);
        let repo = github_repo("https://github.com/me/mod").unwrap();
        assert_eq!(g.download_latest_zip(&repo, "obsidian-commander-", &to).unwrap(), Version::new(0, 2, 0));
        assert_eq!(std::fs::read(&to).unwrap(), b"PK-zip");
        let g = github("v0.2.0", vec![("other.zip", b"x".to_vec())]);
        assert!(matches!(g.download_latest_zip(&repo, "obsidian-commander-", &to), Err(ObsidianError::NoUpdateAsset(_))));
    }

    #[test]
    fn a_release_holding_another_mod_is_refused() {
        let g = github("v1.1.0", vec![("mod.zip", zipped("", &manifest("other.mod", "9.0.0")))]);
        let e = stage(&g, &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateMismatch { .. }), "{e}");
    }

    #[test]
    fn a_zip_older_than_its_tag_is_refused() {
        let g = github("v1.1.0", vec![("mod.zip", zipped("", &manifest("a.mod", "1.0.0")))]);
        let e = stage(&g, &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateMismatch { .. }), "{e}");
    }

    #[test]
    fn nothing_downloads_when_the_mod_is_already_current() {
        let e = stage(&github("v1.0.0", vec![]), &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateMismatch { .. }), "{e}");
    }

    #[test]
    fn a_zip_without_a_mod_is_refused() {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file("readme.txt", SimpleFileOptions::default()).unwrap();
        let g = github("v1.1.0", vec![("mod.zip", z.finish().unwrap().into_inner())]);
        assert!(matches!(stage(&g, &installed("a.mod", "1.0.0")), Err(ObsidianError::NoUpdateAsset(_))));
    }

    #[test]
    fn a_release_needs_one_zip_to_pick() {
        let none = github("v1.1.0", vec![("notes.txt", b"hi".to_vec())]);
        assert!(matches!(stage(&none, &installed("a.mod", "1.0.0")), Err(ObsidianError::NoUpdateAsset(_))));

        let zip = zipped("", &manifest("a.mod", "1.1.0"));
        let two = github("v1.1.0", vec![("one.zip", zip.clone()), ("two.zip", zip.clone())]);
        assert!(matches!(stage(&two, &installed("a.mod", "1.0.0")), Err(ObsidianError::NoUpdateAsset(_))));

        let named = github("v1.1.0", vec![("b.mod-1.1.0.zip", zipped("", &manifest("b.mod", "1.1.0"))), ("a.mod-1.1.0.zip", zip)]);
        assert_eq!(stage(&named, &installed("a.mod", "1.0.0")).unwrap().manifest.id, "a.mod");
    }

    #[test]
    fn a_missing_file_is_a_network_error() {
        let fake = server();
        let base = fake.base.clone();
        fake.ok("/repos/me/mod/releases/latest", format!(r#"{{"tag_name":"v1.1.0","assets":[{{"name":"mod.zip","browser_download_url":"{base}/gone.zip"}}]}}"#));
        let e = stage(&fake.github(), &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateNetwork(_)), "{e}");
    }

    #[test]
    fn a_new_mod_installs_from_the_latest_release() {
        let t = spaced_tempdir();
        let lib = Library::open(t.path()).unwrap();
        let repo = github_repo("https://github.com/me/mod").unwrap();
        let g = github("v0.3.0", vec![("a.mod-0.3.0.zip", zipped("a.mod/", &manifest("a.mod", "0.3.0")))]);
        let staged = stage_new(&g, &repo, "a.mod").unwrap();
        assert!(lib.list().unwrap().is_empty(), "staging leaves the library alone");
        assert_eq!(staged.install(&lib).unwrap(), InstallOutcome::Added);
        assert_eq!(load_manifest(&lib.mod_dir("a.mod")).unwrap().version, Version::new(0, 3, 0));

        let other = github("v0.3.0", vec![("mod.zip", zipped("", &manifest("other.mod", "0.3.0")))]);
        assert!(matches!(stage_new(&other, &repo, "a.mod"), Err(ObsidianError::UpdateMismatch { .. })));
        assert!(matches!(stage_new(&github("v0.3.0", vec![]), &repo, "a.mod"), Err(ObsidianError::NoUpdateAsset(_))));
    }

    #[test]
    fn a_mod_without_a_repository_says_so() {
        let plain: Manifest = serde_json::from_str(r#"{"obsidian":1,"id":"b.mod","version":"1.0.0","name":{"en":"B"}}"#).unwrap();
        assert!(matches!(stage(&github("v1.1.0", vec![]), &plain), Err(ObsidianError::NoUpdateSource(_))));
    }
}
