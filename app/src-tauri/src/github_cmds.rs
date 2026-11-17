//! Commands for the GitHub sign-in and what it enables (obsidian_core::github, share, submit).

use obsidian_core::github::{self, Account, Client, DeviceCode, Message, OpenIssue, Poll, RepoInfo, Report, Templates};
use obsidian_core::share::{self, Downloaded, SyncState};
use obsidian_core::submit::{self, Checked};
use obsidian_core::updates::GitHub;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::State;

use crate::app_settings::AppSettings;
use crate::commands::{CmdResult, Ctx, fresh};
use crate::snapshot::{Snapshot, find};

/// The saved account, read at startup into memory. No network.
#[tauri::command]
pub fn github_account(ctx: State<'_, Ctx>) -> Option<Account> {
    github::load(&ctx.data)
}

/// The saved account checked with GitHub; None when GitHub no longer accepts it.
#[tauri::command]
pub async fn github_verify(ctx: State<'_, Ctx>) -> CmdResult<Option<Account>> {
    Ok(github::verify(&ctx.data, &Client::new())?)
}

/// Starts a sign-in for the access already granted plus `extra` (`gist`, `repo`).
#[tauri::command]
pub async fn github_sign_in_start(ctx: State<'_, Ctx>, extra: Vec<String>) -> CmdResult<DeviceCode> {
    let mut scopes: Vec<String> = github::BASE_SCOPES.iter().map(|s| s.to_string()).collect();
    if let Some(s) = github::saved(&ctx.data) {
        scopes.extend(s.scopes);
    }
    scopes.extend(extra.into_iter().filter(|s| s == github::GIST || s == github::PRIVATE));
    scopes.sort();
    scopes.dedup();
    Ok(github::device_start(&Client::new(), &scopes)?)
}

#[tauri::command]
pub async fn github_sign_in_poll(ctx: State<'_, Ctx>, device_code: String, interval: u64) -> CmdResult<Poll> {
    Ok(github::device_poll(&ctx.data, &Client::new(), &device_code, interval)?)
}

#[tauri::command]
pub fn github_sign_out(ctx: State<'_, Ctx>) -> CmdResult<()> {
    Ok(github::sign_out(&ctx.data)?)
}

#[tauri::command]
pub fn github_revoke_url() -> String {
    github::revoke_url()
}

#[tauri::command]
pub async fn github_repo(repo: String) -> CmdResult<Option<RepoInfo>> {
    Ok(github::repo(&Client::new(), &repo)?)
}

#[tauri::command]
pub async fn github_starred(repo: String) -> CmdResult<bool> {
    Ok(github::starred(&Client::new(), &repo)?)
}

#[tauri::command]
pub async fn github_starred_repos() -> CmdResult<Vec<String>> {
    Ok(github::starred_repos(&Client::new())?)
}

#[tauri::command]
pub async fn github_star(repo: String, on: bool) -> CmdResult<()> {
    Ok(github::set_star(&Client::new(), &repo, on)?)
}

#[tauri::command]
pub async fn github_templates(repo: String) -> CmdResult<Templates> {
    Ok(github::templates(&Client::new(), &repo)?)
}

/// The problem report (versions, mods on, the end of the game's log), to attach to an issue.
#[tauri::command]
pub async fn github_diagnostics(ctx: State<'_, Ctx>) -> CmdResult<String> {
    let portable = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("portable.txt").is_file())).unwrap_or(false);
    let game = find(&AppSettings::load(&ctx.data));
    Ok(obsidian_core::report::build(&ctx.data, game.as_ref(), &obsidian_core::report::AppInfo { version: env!("CARGO_PKG_VERSION"), portable })?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn github_report(ctx: State<'_, Ctx>, repo: String, title: String, body: String, labels: Vec<String>, mod_id: String, mod_name: String) -> CmdResult<Report> {
    Ok(github::create_issue(&ctx.data, &Client::new(), &repo, &title, &body, &labels, &mod_id, &mod_name)?)
}

#[tauri::command]
pub fn github_reports(ctx: State<'_, Ctx>) -> Vec<Report> {
    github::reports(&ctx.data)
}

#[tauri::command]
pub async fn github_reports_refresh(ctx: State<'_, Ctx>) -> CmdResult<Vec<Report>> {
    Ok(github::refresh_reports(&ctx.data, &Client::new())?)
}

#[tauri::command]
pub fn github_report_forget(ctx: State<'_, Ctx>, repo: String, number: u64) -> CmdResult<Vec<Report>> {
    Ok(github::forget_report(&ctx.data, &repo, number)?)
}

#[tauri::command]
pub async fn github_thread(ctx: State<'_, Ctx>, repo: String, number: u64) -> CmdResult<Vec<Message>> {
    Ok(github::thread(&ctx.data, &Client::new(), &repo, number)?)
}

#[tauri::command]
pub async fn github_reply(ctx: State<'_, Ctx>, repo: String, number: u64, body: String) -> CmdResult<Vec<Message>> {
    Ok(github::reply(&ctx.data, &Client::new(), &repo, number, &body)?)
}

#[tauri::command]
pub async fn github_open_issues(repo: String) -> CmdResult<Vec<OpenIssue>> {
    Ok(github::open_issues(&Client::new(), &repo)?)
}

#[tauri::command]
pub async fn sync_state(ctx: State<'_, Ctx>) -> CmdResult<SyncState> {
    Ok(share::sync_state(&ctx.data, &Client::new())?)
}

/// The settings that belong to every PC: appearance and language, never the game folder.
fn portable_settings(s: &AppSettings) -> Value {
    json!({ "theme": s.theme, "scale": s.scale, "text": s.text, "motion": s.motion, "language": s.language })
}

#[tauri::command]
pub async fn sync_upload(ctx: State<'_, Ctx>) -> CmdResult<SyncState> {
    // Reads the profiles and talks to GitHub; changes nothing in the library, so no turn.
    let settings = portable_settings(&AppSettings::load(&ctx.data));
    Ok(share::upload(&ctx.data, &Client::new(), settings)?)
}

#[derive(Serialize)]
pub struct SyncDownloaded {
    snapshot: Snapshot,
    downloaded: Downloaded,
    settings: AppSettings,
}

#[tauri::command]
pub async fn sync_download(ctx: State<'_, Ctx>) -> CmdResult<SyncDownloaded> {
    // The gist and the mods download outside the turn, which only installs and writes.
    let fetched = crate::commands::blocking(|| share::fetch(&ctx.data, &Client::new(), &GitHub::default()))?;
    let _turn = ctx.turn();
    let downloaded = share::apply(&ctx.data, fetched)?;
    let mut s = AppSettings::load(&ctx.data);
    if let Value::Object(remote) = &downloaded.settings {
        let mut merged = serde_json::to_value(&s).map_err(obsidian_core::ObsidianError::from)?;
        for key in ["theme", "scale", "text", "motion", "language"] {
            if let Some(v) = remote.get(key) {
                merged[key] = v.clone();
            }
        }
        if let Ok(next) = serde_json::from_value::<AppSettings>(merged) {
            s = next;
            s.save(&ctx.data)?;
        }
        s = AppSettings::load(&ctx.data);
    }
    Ok(SyncDownloaded { snapshot: fresh(&ctx)?, downloaded, settings: s })
}

#[tauri::command]
pub async fn submit_check(ctx: State<'_, Ctx>, repo: String) -> CmdResult<Checked> {
    Ok(submit::check(&ctx.data, &Client::new(), &GitHub::default(), &repo)?)
}

/// Checks again and, only with no problem left, opens the pull request.
#[tauri::command]
pub async fn submit_send(ctx: State<'_, Ctx>, repo: String, note: String) -> CmdResult<String> {
    let c = Client::new();
    let checked = submit::check(&ctx.data, &c, &GitHub::default(), &repo)?;
    if !checked.problems.is_empty() {
        return Err(obsidian_core::ObsidianError::GitHub("the repository doesn't pass the checks".into()).into());
    }
    Ok(github::open_submission(&c, &checked.repo, &note)?)
}
