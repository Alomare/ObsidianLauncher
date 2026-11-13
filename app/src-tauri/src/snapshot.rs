//! Everything the UI shows, read from core in one go. Every command returns a fresh snapshot, so
//! the UI never keeps a copy of the library or the profile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use obsidian_core::compose::plan_with;
use obsidian_core::deploy::{ChangesWaiting, UncleanEnd, changes_waiting_for, crash_culprit, settings_digest, has_deployed, load_state, loader_deployed, unclean_end};
use obsidian_core::game::{GameInstall, find_game, is_running};
use obsidian_core::gate::{KNOWN_BUILDS, check_build};
use obsidian_core::hash::{display_name, display_text};
use obsidian_core::images;
use obsidian_core::library::{BrokenMod, ImportInfo, Library, Profile, folder_bytes};
use obsidian_core::manifest::{Localized, Manifest};
use obsidian_core::options::{OptionKind, OptionSpec, effective_values, parse_keybinds};
use obsidian_core::{Result, option_layout, profiles, starter};
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
    /// A change needs a deploy (it picks resources, switches a part, or is part of a session mod's mod set).
    pub deploy: bool,
    /// The heading the option shows under (`option_groups`).
    pub group: Option<String>,
    /// The part of the mod the option belongs to.
    pub part: Option<String>,
}

/// One part of a mod (`parts`), as the About tab lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartRow {
    pub id: String,
    /// client or session.
    pub scope: &'static str,
    pub name: Localized,
    pub description: Localized,
    /// False while the toggle option of the part is off.
    pub on: bool,
}

/// A collapsible heading of the Options tab (`option_groups`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRow {
    pub id: String,
    pub label: Localized,
    pub description: Localized,
    /// Folded until the player opens it.
    pub collapsed: bool,
}

/// A live picture of option values (`option_previews`): the SVG template the app fills in.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRow {
    pub id: String,
    pub group: Option<String>,
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub label: Localized,
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
    /// The scope the mod runs with: a mod with parts takes it from the parts that are on, and is
    /// `hybrid` with a client part and a session part on.
    pub scope: &'static str,
    pub on: bool,
    pub parts: Vec<PartRow>,
    pub option_groups: Vec<GroupRow>,
    pub option_previews: Vec<PreviewRow>,
    pub options: Vec<OptionRow>,
    pub keybinds: Vec<KeyRow>,
    /// The game resources the mod changes, as `bundle/name`.
    pub files: Vec<String>,
    pub import: Option<ImportInfo>,
    pub source: Option<String>,
    /// `source` is a repository the mod can update from.
    pub updatable: bool,
    pub image: Option<ImageRef>,
    /// The folder the mod sits in, by id.
    pub folder: Option<String>,
    /// Ids of the mods this one needs.
    pub requires: Vec<String>,
    /// Bytes of the mod's folder in the library.
    pub size: u64,
    /// The last deploy put the mod in the game.
    pub deployed: bool,
    /// Bytes the mod put in the game's patches at the last deploy; `None` when not deployed, or when the
    /// deploy was made before Obsidian counted them.
    pub deployed_size: Option<u64>,
}

/// Folder sizes by mod id, kept while the manifest file is unchanged: every click builds a snapshot,
/// and an install or update rewrites the manifest.
static SIZES: std::sync::Mutex<BTreeMap<String, (Option<std::time::SystemTime>, u64)>> = std::sync::Mutex::new(BTreeMap::new());

fn installed_size(lib: &Library, id: &str) -> u64 {
    let dir = lib.mod_dir(id);
    let stamp = std::fs::metadata(dir.join("obsidian.json")).and_then(|m| m.modified()).ok();
    let mut sizes = SIZES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((s, n)) = sizes.get(id)
        && *s == stamp
        && stamp.is_some()
    {
        return *n;
    }
    let n = folder_bytes(&dir);
    sizes.insert(id.to_string(), (stamp, n));
    n
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
    /// Whether the last deploy put the loader in the game, which a Sandbox save needs.
    pub loader: bool,
    /// The selected Sandbox, when one is.
    pub sandbox: Option<String>,
    pub conflicts: Vec<ConflictRow>,
    pub changes: ChangesWaiting,
    /// Whether a purge has anything to remove.
    pub deployed: bool,
    /// Whether the last deploy put mods in the game, which Play then starts. The Enhanced War Client
    /// mod a war switches on doesn't count: the war needs it.
    pub modded: bool,
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

/// The option rows of one mod, with the player's values.
fn option_rows(m: &Manifest, specs: &[OptionSpec], values: &BTreeMap<String, Value>) -> Vec<OptionRow> {
    specs
        .iter()
        .zip(&m.options)
        .map(|(s, raw)| {
            let (min, max, step) = match &s.kind {
                OptionKind::Number { min, max, step } => (*min, *max, *step),
                _ => (None, None, None),
            };
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
                visible: obsidian_core::options::when_matches(&s.visible_when, values),
                deploy: m.option_needs_deploy(s, values),
                group: s.group.clone(),
                part: s.part.clone(),
            }
        })
        .collect()
}

/// The parts, option groups and previews of one mod. A preview whose template can't be read is left out.
fn layout_rows(lib: &Library, m: &Manifest, values: &BTreeMap<String, Value>) -> Result<(Vec<PartRow>, Vec<GroupRow>, Vec<PreviewRow>)> {
    let parts = m
        .parts
        .iter()
        .map(|p| PartRow { id: p.id.clone(), scope: p.scope.name(), name: p.name.clone(), description: p.description.clone().unwrap_or_default(), on: m.part_on(p, values) })
        .collect();
    let groups = m.option_groups()?.into_iter().map(|g| GroupRow { id: g.id, label: g.label, description: g.description, collapsed: g.collapsed }).collect();
    let dir = lib.mod_dir(&m.id);
    let previews = m
        .option_previews()?
        .into_iter()
        .filter_map(|p| {
            let svg = option_layout::template(&dir, &p)?;
            Some(PreviewRow { id: p.id, group: p.group, svg, width: p.width, height: p.height, label: p.label })
        })
        .collect();
    Ok((parts, groups, previews))
}

/// The scope the row shows: from the parts that are on, `hybrid` with both kinds on.
fn shown_scope(m: &Manifest, values: &BTreeMap<String, Value>) -> &'static str {
    if m.hybrid_with(values) { "hybrid" } else { m.scope_with(values).name() }
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

    // What the last deploy put in the game. A cut-short deploy still has the previous one in place.
    let state = load_state(data).ok().flatten().and_then(|s| if s.incomplete { s.previous.map(|p| *p) } else { Some(s) });
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
        let specs = m.option_specs()?;
        let values = effective_values(&specs, profile.options.get(&m.id));
        let (parts, option_groups, option_previews) = layout_rows(&lib, m, &values)?;
        mods.push(ModRow {
            id: m.id.clone(),
            name: m.name.clone(),
            description: m.description.clone().unwrap_or_default(),
            authors: m.authors.clone(),
            version: if unversioned { None } else { Some(m.version.to_string()) },
            scope: shown_scope(m, &values),
            on: e.enabled,
            parts,
            option_groups,
            option_previews,
            options: option_rows(m, &specs, &values),
            keybinds,
            files: m.resources.iter().map(|r| resource_label(r.bundle.as_deref().unwrap_or_default(), &display_text(&r.name_text))).collect(),
            import: lib.import_info(&m.id),
            source: m.extra.get("source").and_then(Value::as_str).map(str::to_string),
            updatable: obsidian_core::updates::source_of(m).is_some(),
            image: image_ref(images::from_manifest(&lib, m)),
            folder: e.folder.clone(),
            requires: m.requires.clone(),
            size: installed_size(&lib, &m.id),
            deployed: state.as_ref().is_some_and(|s| s.mods.contains(&m.id)),
            deployed_size: state.as_ref().filter(|s| s.mods.contains(&m.id)).and_then(|s| s.sizes.get(&m.id).copied()),
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
                let image = image_ref(images::custom(data, &name));
                ProfileRow { name, on, block_public: p.block_public, guarded: p.block_achievements || p.freeze_progress, image, color: p.color }
            })
            .collect(),
        profiles: profiles::list(data)?,
        block_public: profile.block_public,
        block_achievements: profile.block_achievements,
        freeze_progress: profile.freeze_progress,
        hide_client_mods: profile.hide_client_mods,
        loader: game_install.as_ref().is_some_and(|g| loader_deployed(g, data)),
        sandbox: obsidian_core::saves::active_sandbox(data),
        conflicts,
        changes: changes_waiting_for(&comp, &profile, &settings_digest(&obsidian_core::loader::settings_with(&profile, &manifests)?), data, game_install.as_ref().map(|g| g.contents()).as_deref()).unwrap_or(ChangesWaiting::Waiting),
        deployed: has_deployed(data),
        modded: obsidian_core::deploy::load_state(data)
            .ok()
            .flatten()
            .is_some_and(|s| !s.incomplete && s.mods.iter().any(|m| m != obsidian_core::commander::ENHANCED_MOD)),
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

    #[test]
    fn a_mod_with_parts_groups_and_a_preview_shows_them() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        let d = t.path().join("cam");
        put(&d, "obsidian.json", r#"{"obsidian":1,"id":"a.cam","version":"1.0.0","name":{"en":"Cam"},"scope":"session","entry":"main.lua",
            "parts":[{"id":"view","scope":"client","name":{"en":"View"},"description":{"en":"Any lobby."}},
                     {"id":"range","scope":"session","option":"far","name":{"en":"Range"}}],
            "option_groups":[{"id":"look","label":{"en":"Look"},"collapsed":true}],
            "option_previews":[{"id":"pic","group":"look","template":"pic.svg","width":120,"height":60}],
            "options":[{"id":"fov","type":"number","default":70,"part":"view","group":"look"},
                       {"id":"far","type":"toggle","default":true,"part":"range"}]}"#);
        put(&d, "main.lua", "return true\n");
        put(&d, "pic.svg", r#"<svg xmlns="http://www.w3.org/2000/svg"><text>{{fov}}</text></svg>"#);
        lib.install(&d).unwrap();
        let settings = AppSettings { game_dir: Some(t.path().join("no game here")), ..Default::default() };
        let row = |s: &Snapshot| s.mods.iter().find(|m| m.id == "a.cam").unwrap().clone();

        let s = snapshot(&data, &settings, &mut GateCache::default()).unwrap();
        let cam = row(&s);
        assert_eq!(cam.scope, "hybrid");
        assert_eq!(cam.parts.iter().map(|p| (p.id.as_str(), p.scope, p.on)).collect::<Vec<_>>(), [("view", "client", true), ("range", "session", true)]);
        assert_eq!(cam.parts[0].description["en"], "Any lobby.");
        assert_eq!((cam.option_groups[0].id.as_str(), cam.option_groups[0].collapsed), ("look", true));
        assert_eq!((cam.option_previews[0].group.as_deref(), cam.option_previews[0].width), (Some("look"), 120.0));
        assert!(cam.option_previews[0].svg.contains("{{fov}}"));
        assert_eq!((cam.options[0].group.as_deref(), cam.options[0].part.as_deref(), cam.options[0].deploy), (Some("look"), Some("view"), false));
        assert!(cam.options[1].deploy, "switching a part needs a deploy");

        let mut p = Profile::load(&data).unwrap();
        p.options.entry("a.cam".into()).or_default().insert("far".into(), Value::Bool(false));
        p.save(&data).unwrap();
        let cam = row(&snapshot(&data, &settings, &mut GateCache::default()).unwrap());
        assert_eq!(cam.scope, "client", "only the client part is on");
        assert!(!cam.parts[1].on);
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
