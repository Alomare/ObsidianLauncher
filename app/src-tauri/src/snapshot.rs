//! Everything the UI shows, read from core in one go. Every command returns a fresh snapshot, so
//! the UI never keeps a copy of the library or the profile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use obsidian_core::compose::plan_with;
use obsidian_core::deploy::{ChangesWaiting, UncleanEnd, changes_waiting_for, crash_culprit, settings_digest, has_deployed, unclean_end};
use obsidian_core::game::{GameInstall, find_game, is_running};
use obsidian_core::gate::{KNOWN_BUILDS, check_build};
use obsidian_core::hash::{display_name, display_text};
use obsidian_core::images::{self, Owner};
use obsidian_core::library::{BrokenMod, ImportInfo, Library, Profile};
use obsidian_core::manifest::{Localized, Manifest};
use obsidian_core::options::{OptionKind, effective_values, parse_keybinds};
use obsidian_core::{Result, profiles, starter};
use serde::Serialize;
use serde_json::Value;

use crate::app_settings::AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GameState {
    Supported,
    Unknown,
    Running,
    NotFound,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionRow {
    pub id: String,
    /// toggle, number, choice, key or color.
    pub kind: &'static str,
    pub label: Localized,
    pub description: Localized,
    pub default: Value,
    pub value: Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub choices: Vec<String>,
    /// The label of each choice, in the order of `choices` (empty where the manifest gives none).
    pub choice_labels: Vec<Localized>,
    /// False while `visible_when` doesn't match the other values: the app hides the option.
    pub visible: bool,
    /// A change needs a deploy (it picks resources, or it is part of a session mod's mod set).
    pub deploy: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRow {
    pub id: String,
    pub label: Localized,
    pub default: String,
    /// The key set by the player, if any.
    pub key: Option<String>,
    /// Other enabled mods using the same key.
    pub clash: Vec<String>,
    /// The choice option with the controller input for the same action: the Keys tab shows it in this
    /// row, and the Options tab leaves it out.
    pub pad: Option<String>,
}

/// An image file and a stamp that changes when the file does, so the UI drops a stale copy.
#[derive(Debug, Clone, Serialize)]
pub struct ImageRef {
    pub path: PathBuf,
    pub v: u128,
}

fn image_ref(path: Option<PathBuf>) -> Option<ImageRef> {
    let path = path?;
    let v = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis();
    Some(ImageRef { path, v })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModRow {
    pub id: String,
    pub name: Localized,
    pub description: Localized,
    pub authors: Vec<String>,
    /// `None` for an imported legacy mod, which has no version number.
    pub version: Option<String>,
    pub scope: &'static str,
    pub on: bool,
    pub options: Vec<OptionRow>,
    pub keybinds: Vec<KeyRow>,
    /// The game resources the mod changes, as `bundle/name`.
    pub files: Vec<String>,
    pub import: Option<ImportInfo>,
    pub source: Option<String>,
    /// `source` is a repository the mod can update from.
    pub updatable: bool,
    pub image: Option<ImageRef>,
    /// The image is the player's own, not the mod's.
    pub custom_image: bool,
    /// The folder the mod sits in, by id.
    pub folder: Option<String>,
    /// Ids of the mods this one needs.
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRow {
    pub resource: String,
    /// In load order; the last one wins.
    pub mods: Vec<String>,
    pub winner: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRow {
    pub name: String,
    /// Mods switched on, counting only those in the library.
    pub on: usize,
    /// Blocks other players' session mods in public lobbies.
    pub block_public: bool,
    /// Either of the guardrails that protect the player's own save.
    pub guarded: bool,
    pub image: Option<ImageRef>,
    /// `#rrggbb`, or none for the default gold.
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// In load order.
    pub mods: Vec<ModRow>,
    pub profile: String,
    pub profiles: Vec<String>,
    pub profile_rows: Vec<ProfileRow>,
    /// The active profile's folders.
    pub folders: Vec<obsidian_core::folders::Folder>,
    pub block_public: bool,
    pub block_achievements: bool,
    pub freeze_progress: bool,
    pub hide_client_mods: bool,
    /// The save the active profile plays; None boots whatever is active.
    pub save: Option<String>,
    /// The selected Sandbox, when one is.
    pub sandbox: Option<String>,
    pub conflicts: Vec<ConflictRow>,
    pub changes: ChangesWaiting,
    /// Whether a purge has anything to remove.
    pub deployed: bool,
    pub game: GameState,
    /// The game files that differ from the build Obsidian knows, when it doesn't know this one.
    pub game_changes: Vec<GameChange>,
    pub game_dir: Option<PathBuf>,
    pub crash_culprit: Option<String>,
    /// Mod folders that can't be read, which stay out of the game until they're fixed or removed.
    pub broken: Vec<BrokenMod>,
    /// Profiles that couldn't be read and were set aside.
    pub damaged_profiles: Vec<String>,
    /// The last launch stopped without closing normally. Never set while the game runs.
    pub unclean_end: Option<UncleanEnd>,
    pub portable: bool,
    pub version: &'static str,
}

/// A game file that differs from the build Obsidian knows, by its path under the game folder.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameChange {
    pub path: String,
    pub missing: bool,
}

/// The game files that differ from the build Obsidian knows: empty when it knows the build.
/// Hashing the game files takes a moment, so the answer is kept per folder until `forget`
/// (Settings, "Check again").
#[derive(Default)]
pub struct GateCache(Option<(PathBuf, Vec<GameChange>)>);

impl GateCache {
    fn changes(&mut self, root: &Path) -> Vec<GameChange> {
        if let Some((r, c)) = &self.0
            && r == root
        {
            return c.clone();
        }
        let changes = match check_build(root, KNOWN_BUILDS) {
            Ok(g) if g.build.is_some() => Vec::new(),
            Ok(g) => g.mismatches.into_iter().map(|path| GameChange { missing: !root.join(&path).is_file(), path }).collect(),
            Err(_) => vec![GameChange { path: "contents".into(), missing: true }],
        };
        self.0 = Some((root.to_path_buf(), changes.clone()));
        changes
    }
    pub fn forget(&mut self) {
        self.0 = None;
    }
}

/// The game in the folder chosen in Settings, else the one Steam knows. A chosen folder counts
/// only when it holds the game's `contents` folder.
pub fn find(settings: &AppSettings) -> Option<GameInstall> {
    find_game(settings.game_dir.as_deref()).ok().filter(|g| g.contents().is_dir())
}

/// The path as Windows shows it: Steam stores library paths with `/` and a lowercase drive letter.
fn tidy(p: &Path) -> PathBuf {
    if !cfg!(windows) {
        return p.to_path_buf();
    }
    let s = p.to_string_lossy().replace('/', "\\");
    let mut c = s.chars();
    match (c.next(), s.as_bytes().get(1)) {
        (Some(d), Some(b':')) => PathBuf::from(d.to_ascii_uppercase().to_string() + c.as_str()),
        _ => PathBuf::from(s),
    }
}

/// A game file as people read it: the name, then the last part of the bundle name, as in
/// `lua/settings/status_settings (boot)`. Unknown names stay `#hash`.
fn resource_label(bundle: &str, name: &str) -> String {
    let b = display_text(bundle);
    format!("{name} ({})", b.rsplit('/').next().unwrap_or(&b))
}

fn localized(v: Option<&Value>) -> Localized {
    v.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

/// The option and keybind rows of one mod, with the player's values from the profile.
fn option_rows(m: &Manifest, profile: &Profile) -> Result<Vec<OptionRow>> {
    let specs = m.option_specs()?;
    let values = effective_values(&specs, profile.options.get(&m.id));
    Ok(specs
        .iter()
        .zip(&m.options)
        .map(|(s, raw)| {
            let (min, max, step) = match &s.kind {
                OptionKind::Number { min, max, step } => (*min, *max, *step),
                _ => (None, None, None),
            };
            let deploy = m.resources.iter().any(|r| r.when.contains_key(&s.id)) || (!m.scope.is_client() && !s.client) || m.entry.is_none();
            OptionRow {
                id: s.id.clone(),
                kind: s.kind.name(),
                label: localized(raw.get("label")),
                description: localized(raw.get("description")),
                default: s.default.clone(),
                value: values[&s.id].clone(),
                min,
                max,
                step,
                choices: if let OptionKind::Choice(c) = &s.kind { c.clone() } else { Vec::new() },
                choice_labels: raw.get("choices").and_then(Value::as_array).map(|cs| cs.iter().map(|c| localized(c.get("label"))).collect()).unwrap_or_default(),
                visible: obsidian_core::options::when_matches(&s.visible_when, &values),
                deploy,
            }
        })
        .collect())
}

pub fn snapshot(data: &Path, settings: &AppSettings, gate: &mut GateCache) -> Result<Snapshot> {
    starter::seed(data)?;
    let lib = Library::open(data)?;
    let mut profile = Profile::load(data)?;
    let before = profile.clone();
    // One pass over the library's folders serves the sync, the rows and the mods that can't be read.
    let (installed, broken) = lib.list_and_broken()?;
    profile.sync_with(&lib, &installed)?;
    // Every click comes through here, so the file is only touched when the library changed it.
    if profile != before {
        profile.save(data)?;
    }
    let manifests: BTreeMap<String, Manifest> = installed.into_iter().map(|m| (m.id.clone(), m)).collect();
    let game_install = find(settings);
    let comp = plan_with(&lib, &profile, game_install.as_ref().map(|g| g.contents()).as_deref(), &manifests)?;

    let conflicts: Vec<ConflictRow> = comp
        .conflicts
        .iter()
        .map(|c| {
            let mut mods = c.losers.clone();
            mods.push(c.winner.clone());
            ConflictRow { resource: resource_label(&c.bundle, &display_name(c.name_hash)), mods, winner: c.winner.clone() }
        })
        .collect();

    // Effective key per enabled mod and keybind, to find clashes.
    let mut used: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in profile.mods.iter().filter(|e| e.enabled) {
        if let Some(m) = manifests.get(&e.id) {
            for k in parse_keybinds(&m.keybinds)? {
                let key = profile.keys.get(&m.id).and_then(|s| s.get(&k.id)).cloned().unwrap_or(k.default);
                used.entry(key).or_default().push(m.id.clone());
            }
        }
    }

    let mod_images = images::all_custom(data, Owner::Mod);
    let mut mods = Vec::new();
    for e in &profile.mods {
        let Some(m) = manifests.get(&e.id) else { continue };
        let keybinds = parse_keybinds(&m.keybinds)?
            .into_iter()
            .zip(&m.keybinds)
            .map(|(k, raw)| {
                let key = profile.keys.get(&m.id).and_then(|s| s.get(&k.id)).cloned();
                let effective = key.clone().unwrap_or_else(|| k.default.clone());
                let clash = if e.enabled { used.get(&effective).into_iter().flatten().filter(|id| **id != m.id).cloned().collect() } else { Vec::new() };
                KeyRow { id: k.id, label: localized(raw.get("label")), default: k.default, key, clash, pad: k.pad }
            })
            .collect();
        // An imported legacy mod has no version number unless the Nexus file name gave one.
        let unversioned = m.extra.get("legacy") == Some(&Value::Bool(true)) && m.version == semver::Version::new(0, 0, 0);
        mods.push(ModRow {
            id: m.id.clone(),
            name: m.name.clone(),
            description: m.description.clone().unwrap_or_default(),
            authors: m.authors.clone(),
            version: if unversioned { None } else { Some(m.version.to_string()) },
            scope: m.scope.name(),
            on: e.enabled,
            options: option_rows(m, &profile)?,
            keybinds,
            files: m.resources.iter().map(|r| resource_label(r.bundle.as_deref().unwrap_or_default(), &display_text(&r.name_text))).collect(),
            import: lib.import_info(&m.id),
            source: m.extra.get("source").and_then(Value::as_str).map(str::to_string),
            updatable: obsidian_core::updates::source_of(m).is_some(),
            image: image_ref(mod_images.get(&m.id).cloned().or_else(|| images::from_manifest(&lib, m))),
            custom_image: mod_images.contains_key(&m.id),
            folder: e.folder.clone(),
            requires: m.requires.clone(),
        });
    }

    let game_changes = game_install.as_ref().map(|g| gate.changes(&g.root)).unwrap_or_default();
    let game = match &game_install {
        None => GameState::NotFound,
        Some(_) if is_running() => GameState::Running,
        Some(_) if game_changes.is_empty() => GameState::Supported,
        Some(_) => GameState::Unknown,
    };
    let crash_culprit = game_install.as_ref().and_then(crash_culprit);
    let unclean_end = game_install.as_ref().filter(|_| game != GameState::Running).and_then(unclean_end);
    let portable = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("portable.txt").is_file())).unwrap_or(false);

    Ok(Snapshot {
        mods,
        folders: profile.folders.clone(),
        profile: profiles::active(data)?,
        profile_rows: profiles::list(data)?
            .into_iter()
            .map(|name| {
                let p = Profile::load_named(data, &name).unwrap_or_default();
                let on = p.mods.iter().filter(|e| e.enabled && manifests.contains_key(&e.id)).count();
                let image = image_ref(images::custom(data, Owner::Profile, &name));
                ProfileRow { name, on, block_public: p.block_public, guarded: p.block_achievements || p.freeze_progress, image, color: p.color }
            })
            .collect(),
        profiles: profiles::list(data)?,
        block_public: profile.block_public,
        block_achievements: profile.block_achievements,
        freeze_progress: profile.freeze_progress,
        hide_client_mods: profile.hide_client_mods,
        save: profile.save.clone(),
        sandbox: obsidian_core::saves::active_sandbox(data),
        conflicts,
        changes: changes_waiting_for(&comp, &profile, &settings_digest(&obsidian_core::loader::settings_with(&profile, &manifests)?), data, game_install.as_ref().map(|g| g.contents()).as_deref()).unwrap_or(ChangesWaiting::Waiting),
        deployed: has_deployed(data),
        game,
        game_changes,
        game_dir: game_install.map(|g| tidy(&g.root)),
        crash_culprit,
        broken,
        damaged_profiles: profiles::damaged(data),
        unclean_end,
        portable,
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(dir: &Path, name: &str, text: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn snapshot_lists_mods_with_versions_options_and_conflicts() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        for (id, legacy, version) in [("a.one", false, "1.2.0"), ("legacy.two", true, "0.0.0")] {
            let d = t.path().join(id);
            let extra = if legacy { r#","legacy":true"# } else { "" };
            put(&d, "obsidian.json", &format!(
                r##"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}},"scope":"client","entry":"main.lua"{extra},
                   "options":[{{"id":"fast","type":"toggle","default":false,"label":{{"en":"Fast"}}}},
                              {{"id":"pad_toggle","type":"choice","choices":["thumbs","off"],"default":"thumbs"}}],
                   "keybinds":[{{"id":"toggle","default":"f8","pad":"pad_toggle"}}],
                   "resources":[{{"type":"#0000000000000001","name":"#00000000000000f0","bundle":"00000000000000aa","data":"r.bin"}}]}}"##
            ));
            put(&d, "main.lua", "return true\n");
            put(&d, "r.bin", id);
            lib.install(&d).unwrap();
        }
        let mut p = Profile::load(&data).unwrap();
        p.sync(&lib).unwrap();
        p.set_order(&["legacy.two".into(), "a.one".into()]);
        p.set_enabled("a.one", true).unwrap();
        p.set_enabled("legacy.two", true).unwrap();
        p.options.entry("a.one".into()).or_default().insert("fast".into(), Value::Bool(true));
        p.save(&data).unwrap();
        let settings = AppSettings { game_dir: Some(t.path().join("no game here")), ..Default::default() };
        let s = snapshot(&data, &settings, &mut GateCache::default()).unwrap();

        let a = s.mods.iter().find(|m| m.id == "a.one").unwrap();
        let two = s.mods.iter().find(|m| m.id == "legacy.two").unwrap();
        assert_eq!(a.version.as_deref(), Some("1.2.0"));
        assert_eq!(two.version, None, "imports show no version");
        assert_eq!((a.options[0].value.clone(), a.options[0].label["en"].as_str(), a.options[0].deploy), (Value::Bool(true), "Fast", false));
        assert_eq!(a.keybinds[0].clash, vec!["legacy.two"], "both use f8");
        assert_eq!(a.keybinds[0].pad.as_deref(), Some("pad_toggle"), "the keybind names its controller input");
        assert_eq!(s.conflicts.len(), 1);
        assert_eq!((s.conflicts[0].mods.clone(), s.conflicts[0].winner.as_str()), (vec!["legacy.two".to_string(), "a.one".to_string()], "a.one"));
        let order: Vec<&str> = s.mods.iter().map(|m| m.id.as_str()).filter(|id| !id.starts_with("obsidian.")).collect();
        assert_eq!(order, ["legacy.two", "a.one"]);
        assert_eq!((s.game, s.profile.as_str(), s.changes), (GameState::NotFound, "Default", ChangesWaiting::Waiting));
    }

    /// The snapshot runs after every action, so the time it takes is the delay of every click.
    /// `cargo test --release -p obsidian-app -- --ignored --nocapture`
    #[test]
    #[ignore = "performance; run with --release -- --ignored"]
    fn perf_snapshot_with_200_mods() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        crate::perf_library::build(&data, 200).unwrap();
        let settings = AppSettings { game_dir: Some(t.path().join("no game here")), ..Default::default() };
        let mut times: Vec<f64> = (0..9)
            .map(|_| {
                let t0 = std::time::Instant::now();
                let s = snapshot(&data, &settings, &mut GateCache::default()).unwrap();
                assert_eq!(s.mods.len(), 200 + starter::MODS.len());
                assert!(s.conflicts.len() >= 199 * 4, "neighbours share files");
                t0.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        let median = times[times.len() / 2];
        println!("snapshot, 200 mods: median {median:.1} ms, slowest {:.1} ms", times[times.len() - 1]);
        // 200 mods is far more than people run; 100 ms here keeps a usual library near 10 ms, and
        // a PC four times slower under half a second at this size. The first call can take seconds
        // while Windows scans the new files; a real library pays that once, at import.
        if !cfg!(debug_assertions) {
            assert!(median < 100.0, "snapshot median {median:.1} ms is over the 100 ms budget");
        }
    }
}
