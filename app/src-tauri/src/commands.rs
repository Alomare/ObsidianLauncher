//! Tauri commands: thin calls into obsidian-core. Each one that changes something returns a fresh
//! snapshot.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use obsidian_core::images;
use obsidian_core::deploy::{DeployOptions, deploy as core_deploy, purge as core_purge};
use obsidian_core::game::{GameInstall, ensure_closed};
use obsidian_core::import::import_path;
use obsidian_core::library::{InstallOutcome, Library, Profile};
use obsidian_core::manifest::{Localized, load_manifest};
use obsidian_core::updates::{self, GitHub};
use obsidian_core::{ObsidianError, profiles};
use serde::Serialize;
use tauri::State;

use crate::app_settings::AppSettings;
use crate::snapshot::{GateCache, Snapshot, find, snapshot as read_snapshot};

pub struct Ctx {
    pub data: PathBuf,
    pub gate: Mutex<GateCache>,
    /// Held by each command that reads or changes the library or profile. The commands are async,
    /// so they run off the window's thread and the window stays responsive; this keeps them in turn.
    pub work: Mutex<()>,
    /// Commander's relay and server (commander.rs).
    pub commander: Mutex<crate::commander::Runtime>,
}

impl Ctx {
    pub(crate) fn turn(&self) -> std::sync::MutexGuard<'_, ()> {
        self.work.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// An error for the UI: `code` is the core error's name, which the UI turns into a catalog key
/// (`app.error.<code>`); `message` is the English text, shown when the catalog has no entry.
#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl From<ObsidianError> for CmdError {
    fn from(e: ObsidianError) -> Self {
        let debug = format!("{e:?}");
        let code = debug.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("Other").to_string();
        CmdError { code, message: e.to_string() }
    }
}

impl From<std::io::Error> for CmdError {
    fn from(e: std::io::Error) -> Self {
        CmdError { code: "Io".into(), message: e.to_string() }
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

pub fn fresh(ctx: &Ctx) -> CmdResult<Snapshot> {
    let settings = AppSettings::load(&ctx.data);
    let mut gate = ctx.gate.lock().unwrap_or_else(|p| p.into_inner());
    Ok(read_snapshot(&ctx.data, &settings, &mut gate)?)
}

/// Loads the active profile in step with the library, lets `f` change it, and saves it.
fn with_profile(data: &Path, f: impl FnOnce(&mut Profile) -> obsidian_core::Result<()>) -> CmdResult<()> {
    let lib = Library::open(data)?;
    let mut p = Profile::load(data)?;
    p.sync(&lib)?;
    f(&mut p)?;
    p.save(data)?;
    Ok(())
}

/// The outcome of one dropped or picked path. The UI writes the notice from it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// The file or folder name.
    pub file: String,
    pub ok: bool,
    /// installed, updated or reinstalled.
    pub kind: &'static str,
    pub name: Localized,
    /// `None` for an imported legacy mod, which has no version number.
    pub version: Option<String>,
    pub old_version: Option<String>,
    pub error: Option<CmdError>,
}

#[derive(Debug, Serialize)]
pub struct ImportBatch {
    pub snapshot: Snapshot,
    pub results: Vec<ImportResult>,
}

fn import_one(data: &Path, game: Option<&GameInstall>, path: &Path) -> obsidian_core::Result<ImportResult> {
    let game = game.ok_or_else(|| ObsidianError::GameNotFound(String::new()))?;
    let lib = Library::open(data)?;
    let r = import_path(path, game, &lib)?;
    let m = load_manifest(&lib.mod_dir(&r.id))?;
    let versioned = !r.legacy || r.version != semver::Version::new(0, 0, 0);
    let version = versioned.then(|| r.version.to_string());
    let (kind, old_version) = match r.outcome {
        InstallOutcome::Added => ("installed", None),
        InstallOutcome::Replaced { old_version } if old_version == r.version => ("reinstalled", None),
        InstallOutcome::Replaced { old_version } => ("updated", versioned.then(|| old_version.to_string())),
    };
    Ok(ImportResult { file: String::new(), ok: true, kind, name: m.name, version, old_version, error: None })
}

/// Imports each path in turn; one failure doesn't stop the rest. New mods join the active profile
/// switched off.
pub fn import_all(data: &Path, game: Option<&GameInstall>, paths: &[PathBuf]) -> Vec<ImportResult> {
    let results: Vec<ImportResult> = paths
        .iter()
        .map(|p| {
            let file = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.display().to_string());
            match import_one(data, game, p) {
                Ok(r) => ImportResult { file, ..r },
                Err(e) => ImportResult { file, ok: false, kind: "", name: Localized::new(), version: None, old_version: None, error: Some(e.into()) },
            }
        })
        .collect();
    if results.iter().any(|r| r.ok) {
        let _ = with_profile(data, |_| Ok(()));
    }
    results
}

/// Takes the option values the player changed in the game's pause menu into the active profile,
/// unless the game runs. The caller holds the turn. A failure leaves them for the next try and
/// stops nothing.
fn take_game_options(data: &Path, game: Option<&GameInstall>) {
    if let Some(g) = game {
        let _ = obsidian_core::game_options::take(data, &g.saves_dir, obsidian_core::game::is_running);
    }
}

/// The window asks at startup and whenever the game starts or stops, so values changed in the game
/// show in the mod details once it closes.
#[tauri::command]
pub async fn snapshot(ctx: State<'_, Ctx>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    take_game_options(&ctx.data, find(&AppSettings::load(&ctx.data)).as_ref());
    fresh(&ctx)
}

#[tauri::command]
pub fn get_app_settings(ctx: State<'_, Ctx>) -> AppSettings {
    AppSettings::load(&ctx.data)
}

#[tauri::command]
pub fn set_app_settings(ctx: State<'_, Ctx>, settings: AppSettings) -> CmdResult<AppSettings> {
    // The game folder has its own command, so a window with older settings can't undo it.
    AppSettings { game_dir: AppSettings::load(&ctx.data).game_dir, ..settings }.save(&ctx.data)?;
    Ok(AppSettings::load(&ctx.data))
}

#[tauri::command]
pub async fn set_enabled(ctx: State<'_, Ctx>, ids: Vec<String>, on: bool) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    let lib = Library::open(&ctx.data)?;
    with_profile(&ctx.data, |p| ids.iter().try_for_each(|id| p.switch(&lib, id, on).map(|_| ())))?;
    fresh(&ctx)
}

/// Sets the load order; `moves` puts mods into a folder (or out of any, with null) on the way.
#[tauri::command]
pub async fn set_order(ctx: State<'_, Ctx>, ids: Vec<String>, moves: Option<Vec<(String, Option<String>)>>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    with_profile(&ctx.data, |p| {
        p.arrange(&ids, &moves.unwrap_or_default());
        Ok(())
    })?;
    fresh(&ctx)
}

/// What Sort Mods would do to the active profile's order, without changing it.
#[tauri::command]
pub async fn sort_preview(ctx: State<'_, Ctx>) -> CmdResult<obsidian_core::sort::SortPlan> {
    let _turn = ctx.turn();
    let lib = Library::open(&ctx.data)?;
    let mut p = Profile::load(&ctx.data)?;
    p.sync(&lib)?;
    Ok(obsidian_core::sort::plan(&lib, &p)?)
}

/// Puts the load order in the order Sort Mods showed; `moved` names the mods it moved.
#[tauri::command]
pub async fn apply_sort(ctx: State<'_, Ctx>, ids: Vec<String>, moved: Vec<String>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    with_profile(&ctx.data, |p| {
        p.apply_sort(&ids, &moved);
        Ok(())
    })?;
    fresh(&ctx)
}

#[tauri::command]
pub async fn use_profile(ctx: State<'_, Ctx>, name: String) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    profiles::use_profile(&ctx.data, &name)?;
    // The deploy fingerprint leaves out consent, options and keys, so the game's settings have to
    // follow here, or it keeps the last profile's until the next deploy.
    live_edit(&ctx, |_, _| Ok(()))
}

/// A new profile, empty or a copy of `copy_of`.
#[tauri::command]
pub async fn create_profile(ctx: State<'_, Ctx>, name: String, copy_of: Option<String>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    profiles::create(&ctx.data, &name, copy_of.as_deref())?;
    fresh(&ctx)
}

#[tauri::command]
pub async fn rename_profile(ctx: State<'_, Ctx>, from: String, to: String) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    profiles::rename(&ctx.data, &from, &to)?;
    fresh(&ctx)
}

#[tauri::command]
pub async fn delete_profile(ctx: State<'_, Ctx>, name: String) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    profiles::delete(&ctx.data, &name)?;
    live_edit(&ctx, |_, _| Ok(()))
}

/// Sets the image of the profile `key`, or removes it.
#[tauri::command]
pub async fn set_image(ctx: State<'_, Ctx>, key: String, path: Option<PathBuf>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    images::set(&ctx.data, &key, path.as_deref())?;
    fresh(&ctx)
}

/// A folder holding `ids` (maybe none) at the place of the first of them.
#[tauri::command]
pub async fn create_folder(ctx: State<'_, Ctx>, name: String, color: String, ids: Vec<String>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    with_profile(&ctx.data, |p| {
        p.create_folder(&name, &color, &ids);
        Ok(())
    })?;
    fresh(&ctx)
}

#[tauri::command]
pub async fn edit_folder(ctx: State<'_, Ctx>, id: String, name: Option<String>, color: Option<String>, collapsed: Option<bool>, locked: Option<bool>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    with_profile(&ctx.data, |p| p.edit_folder(&id, name.as_deref(), color.as_deref(), collapsed, locked))?;
    fresh(&ctx)
}

/// Deletes a folder. The mods in it stay in the list, or with `remove_mods` leave the library.
#[tauri::command]
pub async fn delete_folder(ctx: State<'_, Ctx>, id: String, remove_mods: bool) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    let mut members = Vec::new();
    with_profile(&ctx.data, |p| {
        members = p.delete_folder(&id);
        Ok(())
    })?;
    if remove_mods {
        remove_from_library(&ctx.data, &members)?;
    }
    live_edit(&ctx, |_, _| Ok(()))
}

fn remove_from_library(data: &Path, ids: &[String]) -> CmdResult<()> {
    let lib = Library::open(data)?;
    for id in ids {
        lib.remove(id)?;
    }
    Ok(())
}

/// Writes profile `name` with the files of the enabled mods to a ZIP. Returns how many mods went in.
#[tauri::command]
pub async fn export_profile(ctx: State<'_, Ctx>, name: String, dest: PathBuf) -> CmdResult<usize> {
    let _turn = ctx.turn();
    Ok(obsidian_core::profile_zip::export(&ctx.data, &name, &dest)?)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImported {
    pub snapshot: Snapshot,
    pub imported: obsidian_core::profile_zip::Imported,
}

/// Adds the profile in a ZIP from `export_profile`, with the mods it carries.
#[tauri::command]
pub async fn import_profile(ctx: State<'_, Ctx>, path: PathBuf) -> CmdResult<ProfileImported> {
    let _turn = ctx.turn();
    let imported = obsidian_core::profile_zip::import(&ctx.data, &path)?;
    Ok(ProfileImported { snapshot: fresh(&ctx)?, imported })
}

/// Consent belongs to the active profile; it reaches the game through the settings file.
/// Sets the color of a profile, or clears it with `None`.
#[tauri::command]
pub async fn set_profile_color(ctx: State<'_, Ctx>, name: String, color: Option<String>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    profiles::set_color(&ctx.data, &name, color.as_deref())?;
    fresh(&ctx)
}

#[tauri::command]
pub async fn set_block_public(ctx: State<'_, Ctx>, on: bool) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, |_, p| {
        p.block_public = on;
        Ok(())
    })
}

/// The guardrails belong to the active profile too, and reach the game at its next launch.
#[tauri::command]
pub async fn set_block_achievements(ctx: State<'_, Ctx>, on: bool) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, |_, p| {
        p.block_achievements = on;
        Ok(())
    })
}

#[tauri::command]
pub async fn set_freeze_progress(ctx: State<'_, Ctx>, on: bool) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, |_, p| {
        p.freeze_progress = on;
        Ok(())
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// `YYYYMMDD-HHMMSS`, maybe with `-2`.
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub data_dir: PathBuf,
    pub backups: Vec<Backup>,
    /// The folder chosen in Settings; `None` when the game is found through Steam.
    pub chosen_game_dir: Option<PathBuf>,
    /// Where the loader and the mods log; `None` with no game.
    pub logs_dir: Option<PathBuf>,
    /// Patches in the game folder that aren't Obsidian's.
    pub foreign: Vec<obsidian_core::deploy::ForeignPatch>,
    /// Where earlier removals of those patches left their copies.
    pub removed_patches: Option<PathBuf>,
}

/// The folder that holds the logs of the game in `settings`.
fn logs_dir(settings: &AppSettings) -> Option<PathBuf> {
    find(settings).map(|g| obsidian_core::settings::game_side_dir(&g.saves_dir).join("logs"))
}

#[tauri::command]
pub fn system_info(ctx: State<'_, Ctx>) -> SystemInfo {
    let backups = obsidian_core::backup::list_backups(&ctx.data.join("backups"))
        .into_iter()
        .map(|p| Backup { name: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), path: p })
        .collect();
    let settings = AppSettings::load(&ctx.data);
    let foreign = find(&settings).and_then(|g| obsidian_core::deploy::foreign_files(&g, &ctx.data).ok()).unwrap_or_default();
    let kept = ctx.data.join(REMOVED_PATCHES);
    SystemInfo { data_dir: ctx.data.clone(), backups, logs_dir: logs_dir(&settings), foreign, removed_patches: kept.is_dir().then_some(kept), chosen_game_dir: settings.game_dir }
}

/// The folder in the data folder that keeps the patches taken out with `remove_other_patches`.
const REMOVED_PATCHES: &str = "removed-patches";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherPatchesRemoved {
    pub snapshot: Snapshot,
    pub removed: usize,
    pub kept_in: Option<PathBuf>,
}

/// Takes the patches that aren't Obsidian's out of the game folder, into a dated folder under
/// `removed-patches`, so they can be put back by hand.
#[tauri::command]
pub async fn remove_other_patches(ctx: State<'_, Ctx>) -> CmdResult<OtherPatchesRemoved> {
    let _turn = ctx.turn();
    let game = game_of(&ctx)?;
    ensure_closed()?;
    let keep = ctx.data.join(REMOVED_PATCHES).join(obsidian_core::backup::stamp());
    let r = obsidian_core::deploy::remove_foreign(&game, &ctx.data, &keep)?;
    Ok(OtherPatchesRemoved { snapshot: fresh(&ctx)?, removed: r.removed.len(), kept_in: r.kept_in })
}

/// The text for a bug report, for the clipboard.
#[tauri::command]
pub fn support_report(ctx: State<'_, Ctx>) -> CmdResult<String> {
    let portable = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("portable.txt").is_file())).unwrap_or(false);
    let game = find(&AppSettings::load(&ctx.data));
    Ok(obsidian_core::report::build(&ctx.data, game.as_ref(), &obsidian_core::report::AppInfo { version: env!("CARGO_PKG_VERSION"), portable })?)
}

/// Uses the game in `path`, or finds it through Steam again when `path` is null. A folder without
/// the game's `contents` folder is refused.
#[tauri::command]
pub async fn set_game_dir(ctx: State<'_, Ctx>, path: Option<PathBuf>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    if let Some(p) = &path
        && !p.join("contents").is_dir()
    {
        return Err(ObsidianError::GameNotFound(p.display().to_string()).into());
    }
    let mut s = AppSettings::load(&ctx.data);
    s.game_dir = path;
    s.save(&ctx.data)?;
    ctx.gate.lock().unwrap_or_else(|p| p.into_inner()).forget();
    fresh(&ctx)
}

/// Checks the game build again (the answer is kept per folder otherwise).
#[tauri::command]
pub async fn check_game(ctx: State<'_, Ctx>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    ctx.gate.lock().unwrap_or_else(|p| p.into_inner()).forget();
    fresh(&ctx)
}

/// Opens a folder the app knows in Explorer: the game, the data folder, the logs or one save backup.
#[tauri::command]
pub fn open_folder(ctx: State<'_, Ctx>, app: tauri::AppHandle, path: PathBuf) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let settings = AppSettings::load(&ctx.data);
    let known = path.starts_with(&ctx.data) || find(&settings).is_some_and(|g| path == g.root) || logs_dir(&settings).is_some_and(|d| path == d);
    if !known {
        return Err(CmdError { code: "Io".into(), message: format!("not a folder Obsidian opens: {}", path.display()) });
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| CmdError { code: "Io".into(), message: e.to_string() })
}

#[tauri::command]
pub async fn import_paths(ctx: State<'_, Ctx>, paths: Vec<PathBuf>) -> CmdResult<ImportBatch> {
    let _turn = ctx.turn();
    let game = find(&AppSettings::load(&ctx.data));
    let results = import_all(&ctx.data, game.as_ref(), &paths);
    Ok(ImportBatch { snapshot: fresh(&ctx)?, results })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    pub id: String,
    /// The newer version; `None` when the mod is up to date or the check failed.
    pub version: Option<String>,
    pub error: Option<CmdError>,
}

/// What a version of Obsidian changes, as the GitHub release reads now.
#[tauri::command]
pub async fn release_notes(version: String) -> CmdResult<Option<String>> {
    let Some(repo) = updates::github_repo(APP_SOURCE) else { return Ok(None) };
    Ok(GitHub::default().notes(&repo, &format!("v{version}"))?)
}

/// Every published version of Obsidian, for the changelog.
#[tauri::command]
pub async fn release_list() -> CmdResult<Vec<updates::ListedRelease>> {
    let Some(repo) = updates::github_repo(APP_SOURCE) else { return Ok(Vec::new()) };
    Ok(GitHub::default().releases(&repo)?)
}

/// One page of releases from the repository a mod names, for the changelog in the mod's details.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePage {
    pub releases: Vec<updates::ListedRelease>,
    /// GitHub may have older releases on the next page.
    pub more: bool,
}

/// The releases of a mod's `source`, newest first, `page` counted from 1. Only reading the
/// manifest takes a turn; the network doesn't.
#[tauri::command]
pub async fn mod_releases(ctx: State<'_, Ctx>, id: String, page: u32) -> CmdResult<ReleasePage> {
    let m = {
        let _turn = ctx.turn();
        load_manifest(&Library::open(&ctx.data)?.mod_dir(&id))?
    };
    let Some(repo) = updates::source_of(&m) else { return Ok(ReleasePage { releases: Vec::new(), more: false }) };
    let (releases, more) = GitHub::default().releases_page(&repo, page, 30)?;
    Ok(ReleasePage { releases, more })
}

/// Asks GitHub about the mods in `ids` that name a repository. Only reading the library takes a
/// turn, so the other commands don't wait for the network. Every answer is kept in the data folder;
/// the check at startup (`fresh: false`) takes one from the last few hours instead of asking again,
/// and a check the player starts always asks.
#[tauri::command]
pub async fn check_mod_updates(ctx: State<'_, Ctx>, ids: Vec<String>, fresh: Option<bool>) -> CmdResult<Vec<ModUpdate>> {
    let mut mods = {
        let _turn = ctx.turn();
        Library::open(&ctx.data)?.list()?
    };
    mods.retain(|m| ids.contains(&m.id));
    let trust = (!fresh.unwrap_or(true)).then_some(updates::TRUST_SECS);
    let checked = updates::check_remembering(&GitHub::default(), &mods, &ctx.data.join(updates::MEMORY_FILE), trust, updates::now_secs());
    Ok(checked
        .into_iter()
        .map(|c| match c.result {
            Ok(v) => ModUpdate { id: c.id, version: v.map(|v| v.to_string()), error: None },
            Err(e) => ModUpdate { id: c.id, version: None, error: Some(e.into()) },
        })
        .collect())
}

/// Installs the latest release of a mod. The download runs outside the turn, as in
/// `check_mod_updates`, and the mod is replaced at the end.
#[tauri::command]
pub async fn update_mod(ctx: State<'_, Ctx>, id: String) -> CmdResult<ImportBatch> {
    let current = {
        let _turn = ctx.turn();
        load_manifest(&Library::open(&ctx.data)?.mod_dir(&id))?
    };
    let staged = updates::stage(&GitHub::default(), &current)?;
    let _turn = ctx.turn();
    staged.install(&Library::open(&ctx.data)?)?;
    with_profile(&ctx.data, |_| Ok(()))?;
    let result = ImportResult {
        file: id,
        ok: true,
        kind: "updated",
        name: staged.manifest.name.clone(),
        version: Some(staged.manifest.version.to_string()),
        old_version: Some(current.version.to_string()),
        error: None,
    };
    Ok(ImportBatch { snapshot: fresh(&ctx)?, results: vec![result] })
}

/// Saves a change to options or keys and writes the settings file the loader reads, so the change
/// reaches the game at the next launch (also while the game runs).
fn live_edit(ctx: &Ctx, f: impl FnOnce(&Library, &mut Profile) -> obsidian_core::Result<()>) -> CmdResult<Snapshot> {
    let lib = Library::open(&ctx.data)?;
    let mut p = Profile::load(&ctx.data)?;
    p.sync(&lib)?;
    f(&lib, &mut p)?;
    p.save(&ctx.data)?;
    if let Some(g) = find(&AppSettings::load(&ctx.data)) {
        obsidian_core::edit::write_settings(&lib, &p, &g.saves_dir)?;
    }
    fresh(ctx)
}

/// `value` null resets the option to the default.
#[tauri::command]
pub async fn set_option(ctx: State<'_, Ctx>, id: String, option: String, value: Option<serde_json::Value>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, |lib, p| obsidian_core::edit::set_option(lib, p, &id, &option, value.as_ref()).map(|_| ()))
}

/// `key` null resets the keybind to the mod's default.
#[tauri::command]
pub async fn set_key(ctx: State<'_, Ctx>, id: String, keybind: String, key: Option<String>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, |lib, p| obsidian_core::edit::set_key(lib, p, &id, &keybind, key.as_deref()))
}

/// Deletes mods from the library; they leave every profile.
#[tauri::command]
pub async fn remove_mods(ctx: State<'_, Ctx>, ids: Vec<String>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    remove_from_library(&ctx.data, &ids)?;
    live_edit(&ctx, |_, _| Ok(()))
}

/// Puts the starter pack back as it came, in the active profile.
#[tauri::command]
pub async fn restore_starter_pack(ctx: State<'_, Ctx>) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, obsidian_core::starter::restore)
}

const STEAM_LAUNCH: &str = "steam://rungameid/394510";
const APP_SOURCE: &str = "https://github.com/Alomare/ObsidianLauncher";

fn game_of(ctx: &Ctx) -> CmdResult<GameInstall> {
    Ok(find(&AppSettings::load(&ctx.data)).ok_or_else(|| ObsidianError::GameNotFound(String::new()))?)
}

/// Readies the active save with `prepare`, then calls `launch`. The game runs what was last
/// deployed, even with changes waiting.
pub fn play_with(game: Option<&GameInstall>, prepare: impl FnOnce(&GameInstall) -> CmdResult<()>, launch: impl FnOnce() -> CmdResult<()>) -> CmdResult<()> {
    let game = game.ok_or_else(|| ObsidianError::GameNotFound(String::new()))?;
    ensure_closed()?;
    prepare(game)?;
    launch()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployDone {
    pub snapshot: Snapshot,
    /// Mods written into the game.
    pub mods: usize,
    pub backup: bool,
    /// The game already had this deploy; nothing was written.
    pub unchanged: bool,
    /// Option values or keys changed since the last deploy.
    pub settings: bool,
    /// The loader is in the game after this deploy, as a Sandbox save needs.
    pub loader: bool,
}

#[tauri::command]
pub async fn deploy(app: tauri::AppHandle, ctx: State<'_, Ctx>) -> CmdResult<DeployDone> {
    use tauri::Emitter;
    let _turn = ctx.turn();
    let game = game_of(&ctx)?;
    ensure_closed()?;
    // An option changed in the game can pick resources, so it goes into this deploy.
    take_game_options(&ctx.data, Some(&game));
    let tell = move |p: &obsidian_core::deploy::DeployProgress| {
        let _ = app.emit("deploy-progress", p);
    };
    let r = core_deploy(&game, &ctx.data, &DeployOptions { progress: Some(&tell), ..DeployOptions::default() })?;
    let snapshot = fresh(&ctx)?;
    Ok(DeployDone { mods: if r.patches.is_empty() { 0 } else { snapshot.mods.iter().filter(|m| m.on).count() }, backup: r.backup.is_some(), unchanged: r.unchanged, settings: r.settings_changed, loader: snapshot.loader, snapshot })
}

/// The Saves tab's deploy: the loader alone, with every mod left out, so a Sandbox save works in a
/// game with no mods. The Mods tab's deploy puts the mods back.
#[tauri::command]
pub async fn deploy_loader(app: tauri::AppHandle, ctx: State<'_, Ctx>) -> CmdResult<DeployDone> {
    use tauri::Emitter;
    let _turn = ctx.turn();
    let game = game_of(&ctx)?;
    ensure_closed()?;
    let tell = move |p: &obsidian_core::deploy::DeployProgress| {
        let _ = app.emit("deploy-progress", p);
    };
    let r = core_deploy(&game, &ctx.data, &DeployOptions { progress: Some(&tell), loader_only: true, ..DeployOptions::default() })?;
    let snapshot = fresh(&ctx)?;
    Ok(DeployDone { mods: 0, backup: r.backup.is_some(), unchanged: r.unchanged, settings: r.settings_changed, loader: snapshot.loader, snapshot })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeDone {
    pub snapshot: Snapshot,
    pub removed: usize,
    /// Files another tool changed after Obsidian wrote them; left in place.
    pub kept: usize,
}

#[tauri::command]
pub async fn purge(ctx: State<'_, Ctx>) -> CmdResult<PurgeDone> {
    let _turn = ctx.turn();
    let game = game_of(&ctx)?;
    ensure_closed()?;
    let r = core_purge(&game, &ctx.data)?;
    Ok(PurgeDone { snapshot: fresh(&ctx)?, removed: r.removed.len(), kept: r.kept_changed.len() })
}

#[tauri::command]
pub async fn play(ctx: State<'_, Ctx>, app: tauri::AppHandle) -> CmdResult<Snapshot> {
    use tauri_plugin_opener::OpenerExt;
    let _turn = ctx.turn();
    let game = find(&AppSettings::load(&ctx.data));
    let prepare = |g: &GameInstall| -> CmdResult<()> {
        obsidian_core::saves::prepare_play(&ctx.data, g, obsidian_core::game::is_running)?;
        Ok(())
    };
    play_with(game.as_ref(), prepare, || {
        app.opener().open_url(STEAM_LAUNCH, None::<&str>).map_err(|e| CmdError { code: "Launch".into(), message: e.to_string() })
    })?;
    fresh(&ctx)
}

pub(crate) fn saves_of(ctx: &Ctx) -> CmdResult<obsidian_core::saves::Saves> {
    let g = game_of(ctx)?;
    Ok(obsidian_core::saves::Saves::open(&ctx.data, &g, obsidian_core::game::is_running)?)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavesDone {
    pub saves: obsidian_core::saves::SaveList,
    /// What changed for the whole app: the Deploy state and the settings shown.
    pub snapshot: Snapshot,
}

fn saves_done(ctx: &Ctx, s: &obsidian_core::saves::Saves) -> CmdResult<SavesDone> {
    Ok(SavesDone { saves: s.list()?, snapshot: fresh(ctx)? })
}

/// The active Steam account's saves. The first call adopts the game's files as an Official save.
#[tauri::command]
pub async fn saves_list(ctx: State<'_, Ctx>) -> CmdResult<obsidian_core::saves::SaveList> {
    let _turn = ctx.turn();
    Ok(saves_of(&ctx)?.list()?)
}

/// Selects a save. Refused while the game runs.
#[tauri::command]
pub async fn saves_activate(ctx: State<'_, Ctx>, id: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.activate(&id)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_create_official(ctx: State<'_, Ctx>, name: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.create_official(&name)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_snapshot(ctx: State<'_, Ctx>, id: String, name: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.snapshot(&id, &name)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_create_sandbox(ctx: State<'_, Ctx>, name: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.create_sandbox(&name)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_duplicate(ctx: State<'_, Ctx>, id: String, name: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.duplicate(&id, &name)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_rename(ctx: State<'_, Ctx>, id: String, name: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.rename(&id, &name)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_delete(ctx: State<'_, Ctx>, id: String) -> CmdResult<SavesDone> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    s.delete(&id)?;
    saves_done(&ctx, &s)
}

#[tauri::command]
pub async fn saves_export(ctx: State<'_, Ctx>, id: String, dest: PathBuf) -> CmdResult<()> {
    let _turn = ctx.turn();
    saves_of(&ctx)?.export(&id, &dest)?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImported {
    pub id: String,
    pub saves: obsidian_core::saves::SaveList,
    pub snapshot: Snapshot,
}

#[tauri::command]
pub async fn saves_import(ctx: State<'_, Ctx>, path: PathBuf, name: Option<String>) -> CmdResult<SaveImported> {
    let _turn = ctx.turn();
    let s = saves_of(&ctx)?;
    let id = s.import(&path, name.as_deref())?;
    Ok(SaveImported { id, saves: s.list()?, snapshot: fresh(&ctx)? })
}

#[tauri::command]
pub async fn saves_editor_state(ctx: State<'_, Ctx>, id: String) -> CmdResult<obsidian_core::saves::editor::EditorState> {
    let _turn = ctx.turn();
    Ok(saves_of(&ctx)?.editor_state(&id)?)
}

#[tauri::command]
pub async fn saves_apply_edits(ctx: State<'_, Ctx>, id: String, edits: Vec<obsidian_core::saves::editor::Edit>) -> CmdResult<obsidian_core::saves::editor::EditorState> {
    let _turn = ctx.turn();
    Ok(saves_of(&ctx)?.apply_edits(&id, &edits)?)
}

/// The lobby info guardrail: leave client mods out of the lobby info this player hosts.
#[tauri::command]
pub async fn set_hide_client_mods(ctx: State<'_, Ctx>, on: bool) -> CmdResult<Snapshot> {
    let _turn = ctx.turn();
    live_edit(&ctx, |_, p| {
        p.hide_client_mods = on;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_the_variant_name() {
        let e: CmdError = ObsidianError::ProfileExists("A".into()).into();
        assert_eq!((e.code.as_str(), e.message.as_str()), ("ProfileExists", "a profile named A already exists"));
        let e: CmdError = ObsidianError::LastProfile.into();
        assert_eq!(e.code, "LastProfile");
    }

    /// A game folder `find_game` takes on every system: Linux wants a Proton prefix beside it.
    fn fake_game(library: &std::path::Path) -> GameInstall {
        std::fs::create_dir_all(library.join("steamapps/compatdata/394510/pfx")).unwrap();
        let root = library.join(obsidian_core::game::STEAM_DIR);
        std::fs::create_dir_all(root.join("contents")).unwrap();
        obsidian_core::game::find_game(Some(&root)).unwrap()
    }

    fn mod_folder(dir: &std::path::Path, version: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("obsidian.json"), format!(r#"{{"obsidian":1,"id":"a.drop","version":"{version}","name":{{"en":"Drop"}},"scope":"client","entry":"main.lua"}}"#)).unwrap();
        std::fs::write(dir.join("main.lua"), "return true
").unwrap();
    }

    #[test]
    fn import_paths_reports_a_bad_drop() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let game = fake_game(t.path());
        let txt = t.path().join("notes.txt");
        std::fs::write(&txt, "hello").unwrap();
        let empty = t.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let before = Library::open(&data).unwrap().list().unwrap().len();

        let r = import_all(&data, Some(&game), &[txt.clone(), empty]);
        let codes: Vec<&str> = r.iter().map(|r| r.error.as_ref().map_or("", |e| e.code.as_str())).collect();
        assert_eq!(codes, ["UnsupportedArchive", "NothingToImport"]);
        assert!(r.iter().all(|r| !r.ok));
        assert_eq!((r[0].file.as_str(), r[1].file.as_str()), ("notes.txt", "empty"));
        assert_eq!(Library::open(&data).unwrap().list().unwrap().len(), before, "the library is unchanged");
        assert_eq!(import_all(&data, None, &[txt])[0].error.as_ref().unwrap().code, "GameNotFound");
    }

    #[test]
    fn import_paths_says_installed_updated_or_reinstalled() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let game = fake_game(t.path());
        let (v1, v2) = (t.path().join("v1"), t.path().join("v2"));
        mod_folder(&v1, "1.0.0");
        mod_folder(&v2, "1.1.0");
        let kinds = |p: &std::path::Path| {
            let r = import_all(&data, Some(&game), &[p.to_path_buf()]).remove(0);
            (r.ok, r.kind, r.name["en"].clone(), r.version, r.old_version)
        };
        assert_eq!(kinds(&v1), (true, "installed", "Drop".into(), Some("1.0.0".into()), None));
        assert_eq!(kinds(&v2), (true, "updated", "Drop".into(), Some("1.1.0".into()), Some("1.0.0".into())));
        assert_eq!(kinds(&v2).1, "reinstalled");
        let profile = Profile::load(&data).unwrap();
        assert!(profile.mods.iter().any(|m| m.id == "a.drop" && !m.enabled), "new mods join the profile switched off");
    }

    #[test]
    fn play_launches_without_deploying() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let game = fake_game(t.path());
        let launched = std::cell::Cell::new(false);
        let launch = || {
            launched.set(true);
            Ok(())
        };

        // A mod switched on leaves changes waiting. A deploy would fail on this fake game (no known build).
        mod_folder(&t.path().join("m"), "1.0.0");
        import_all(&data, Some(&game), &[t.path().join("m")]);
        with_profile(&data, |p| p.set_enabled("a.drop", true)).unwrap();
        play_with(Some(&game), |_| Ok(()), launch).unwrap();
        assert!(launched.replace(false));
        assert!(!obsidian_core::deploy::has_deployed(&data), "the changes stay waiting");

        assert_eq!(play_with(None, |_| Ok(()), launch).unwrap_err().code, "GameNotFound");
        assert!(!launched.get());
    }
}
