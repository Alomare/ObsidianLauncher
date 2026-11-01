//! Deploy writes Obsidian's patches next to the game's bundles; purge removes exactly those.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backup::backup_game_saves;
use crate::bundle::{self, Bundle, Entry, Variant};
use crate::compose::{Composition, Conflict, compose};
use crate::game::GameInstall;
use crate::gate::{GateReport, KNOWN_BUILDS, KnownBuild, check_build, sha256_file};
use crate::library::{Library, Profile};
use crate::paths::{rename_retrying, write_atomic};
use crate::{ObsidianError, Result, loader, settings};

pub const STATE: &str = "deploy-state.json";
const TMP: &str = "obsidian-tmp";
/// A patch of the last deploy, moved aside while the new ones go in.
const OLD: &str = "obsidian-old";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployedFile {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployState {
    pub build: String,
    pub files: Vec<DeployedFile>,
    pub mods: Vec<String>,
    pub time: String,
    /// `fingerprint` of what was deployed; missing in states written before it existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// Set while the patches are being swapped: if it's still there, a deploy was cut short.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
    /// The deploy being replaced, kept until the swap is done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<Box<DeployState>>,
}

/// Whether the game holds what the active profile would deploy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChangesWaiting {
    UpToDate,
    Waiting,
    /// Nothing deployed and nothing to deploy.
    Vanilla,
}

pub struct DeployOptions<'a> {
    pub known_builds: &'a [KnownBuild],
    pub keep_backups: usize,
    /// The loader sources; `None` finds them with `loader::find_loader_dir`.
    pub loader_dir: Option<PathBuf>,
}

impl Default for DeployOptions<'static> {
    fn default() -> Self {
        DeployOptions { known_builds: KNOWN_BUILDS, keep_backups: 10, loader_dir: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeployReport {
    pub patches: Vec<PathBuf>,
    pub conflicts: Vec<Conflict>,
    pub foreign: Vec<PathBuf>,
    pub backup: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PurgeReport {
    pub removed: Vec<PathBuf>,
    pub kept_changed: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub gate: GateReport,
    pub deployed: Option<DeployState>,
    pub foreign: Vec<PathBuf>,
    /// The mod that was starting when the game last closed or crashed, from the loader log.
    pub crash_culprit: Option<String>,
}

/// `(bundle, N)` for a file named `<16 hex>.patch_<N>`.
fn patch_name(name: &str) -> Option<(String, u32)> {
    // NTFS ignores case, so `...AA.patch_0` and `...aa.patch_0` are the same file.
    let lower = name.to_ascii_lowercase();
    let (bundle, n) = lower.split_once(".patch_")?;
    let hex = bundle.len() == 16 && bundle.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !hex || n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((bundle.to_string(), n.parse().ok()?))
}

/// Where a recorded patch is now. Patches always sit in `contents`, and the path in the state can be
/// spelled differently from the game's path today, or carry an old drive letter.
fn here(contents: &Path, f: &DeployedFile) -> PathBuf {
    f.path.file_name().map_or_else(|| f.path.clone(), |n| contents.join(n))
}

/// `path` with `.<suffix>` added.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{suffix}"));
    PathBuf::from(name)
}

fn write_state(data_dir: &Path, state: &DeployState) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    write_atomic(&data_dir.join(STATE), (serde_json::to_string_pretty(state)? + "\n").as_bytes())?;
    Ok(())
}

/// Puts the game back as it was before a deploy that a crash or a killed app cut short.
fn finish_interrupted(contents: &Path, data_dir: &Path) -> Result<()> {
    let Some(state) = load_state(data_dir)? else { return Ok(()) };
    if !state.incomplete {
        return Ok(());
    }
    for f in &state.files {
        let p = here(contents, f);
        if p.is_file() && sha256_file(&p)? == f.sha256 {
            std::fs::remove_file(&p)?;
        }
    }
    for f in state.previous.iter().flat_map(|p| &p.files) {
        let place = here(contents, f);
        let old = beside(&place, OLD);
        if old.is_file() {
            rename_retrying(&old, &place)?;
        }
    }
    match state.previous {
        Some(p) => write_state(data_dir, &p),
        None => Ok(std::fs::remove_file(data_dir.join(STATE))?),
    }
}

/// Clears what an interrupted write left in the game folder. None of it is a patch the game reads.
fn clear_leftovers(contents: &Path) -> Result<()> {
    if !contents.is_dir() {
        return Ok(());
    }
    for e in std::fs::read_dir(contents)? {
        let p = e?.path();
        if p.is_file() && p.extension().is_some_and(|x| x == TMP || x == OLD) {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}

/// Every regular `*.patch_N` file in `contents` that the recorded state did not write, going by name:
/// the same folder can be spelled two ways.
fn foreign_patches(contents: &Path, state: Option<&DeployState>) -> Result<Vec<(String, u32, PathBuf)>> {
    let recorded = state.into_iter().flat_map(|s| s.files.iter().chain(s.previous.iter().flat_map(|p| &p.files)));
    let ours: Vec<String> = recorded.filter_map(|f| f.path.file_name()).map(|n| n.to_string_lossy().to_ascii_lowercase()).collect();
    let mut out = Vec::new();
    if !contents.is_dir() {
        return Ok(out);
    }
    for e in std::fs::read_dir(contents)? {
        let e = e?;
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if !e.file_type()?.is_file() || ours.contains(&name) {
            continue;
        }
        if let Some((bundle, n)) = patch_name(&name) {
            out.push((bundle, n, p));
        }
    }
    out.sort();
    Ok(out)
}

fn remove_all<'a>(paths: impl Iterator<Item = &'a PathBuf>) {
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}

pub fn load_state(data_dir: &Path) -> Result<Option<DeployState>> {
    match std::fs::read_to_string(data_dir.join(STATE)) {
        Ok(t) => Ok(Some(serde_json::from_str(&t)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Whether a purge has anything to remove. A state that can't be read counts, as purge is what deals with it.
pub fn has_deployed(data_dir: &Path) -> bool {
    match load_state(data_dir) {
        Ok(Some(s)) => s.incomplete || !s.files.is_empty(),
        Ok(None) => false,
        Err(_) => true,
    }
}

/// One hash of what a deploy of `comp` writes: every planned resource and, when the loader goes
/// in, the loader's inputs. Live option values and keys reach the game through `settings.txt`, so
/// they are not part of it.
pub fn fingerprint(comp: &Composition, profile: &Profile, loader_dir: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for (bundle, planned) in &comp.bundles {
        for p in planned {
            h.update(format!("{bundle} {:016x} {:016x} {} {}
", p.type_hash, p.name_hash, p.flags, p.owner).as_bytes());
            h.update(p.digest);
        }
    }
    if loader::needed(comp, profile) {
        h.update(b"loader
");
        h.update(loader::build_inputs(loader_dir, comp)?);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// `contents` is the game's folder of bundles, to check that the recorded patches are still there.
pub fn changes_waiting(lib: &Library, profile: &Profile, data_dir: &Path, contents: Option<&Path>) -> Result<ChangesWaiting> {
    let mut profile = profile.clone();
    profile.sync(lib)?;
    let comp = compose(lib, &profile)?;
    changes_waiting_for(&comp, &profile, data_dir, contents)
}

/// `changes_waiting` for a composition already made from `profile`, synced with the library. A deploy
/// that was cut short, or patches that are gone (a reinstall, another copy of the game), count as
/// changes waiting: Play would otherwise start the game without the mods.
pub fn changes_waiting_for(comp: &Composition, profile: &Profile, data_dir: &Path, contents: Option<&Path>) -> Result<ChangesWaiting> {
    let (comp, profile) = (comp, profile);
    let state = load_state(data_dir)?;
    let deployed_nothing = state.as_ref().is_none_or(|s| s.files.is_empty());
    if comp.bundles.is_empty() && !loader::needed(comp, profile) {
        return Ok(if deployed_nothing { ChangesWaiting::Vanilla } else { ChangesWaiting::Waiting });
    }
    let now = fingerprint(comp, profile, &loader::find_loader_dir()?)?;
    let whole = state.as_ref().is_some_and(|s| !s.incomplete && contents.is_none_or(|c| s.files.iter().all(|f| here(c, f).is_file())));
    Ok(match state.and_then(|s| s.fingerprint) {
        Some(f) if f == now && whole => ChangesWaiting::UpToDate,
        _ => ChangesWaiting::Waiting,
    })
}

pub fn deploy(game: &GameInstall, data_dir: &Path, opts: &DeployOptions) -> Result<DeployReport> {
    let contents = game.contents();
    let gate = check_build(&game.root, opts.known_builds)?;
    let build = gate.build.clone().ok_or_else(|| ObsidianError::UnsupportedBuild(gate.clone()))?;
    finish_interrupted(&contents, data_dir)?;
    clear_leftovers(&contents)?;
    let previous = load_state(data_dir)?;
    let lib = Library::open(data_dir)?;
    let mut profile = Profile::load(data_dir)?;
    profile.sync(&lib)?;
    let mut comp = compose(&lib, &profile)?;
    let loader_dir = match &opts.loader_dir {
        Some(d) => d.clone(),
        None => loader::find_loader_dir()?,
    };
    let fingerprint = fingerprint(&comp, &profile, &loader_dir)?;
    if loader::needed(&comp, &profile) {
        let dir = &loader_dir;
        let planned = loader::build(dir, &comp)?;
        let slot = comp.bundles.entry(loader::BUNDLE.to_string()).or_default();
        for p in planned {
            slot.retain(|q| (q.type_hash, q.name_hash) != (p.type_hash, p.name_hash));
            slot.push(p);
        }
    }
    let settings = loader::settings_for(&lib, &profile)?;

    // Plan every slot before changing anything, so a refusal leaves the previous deploy in place.
    let foreign = foreign_patches(&contents, previous.as_ref())?;
    let mut plan = Vec::new();
    for bundle in comp.bundles.keys() {
        let base = contents.join(bundle);
        if !base.is_file() {
            return Err(ObsidianError::UnknownBundle(bundle.clone()));
        }
        let taken: Vec<u32> = foreign.iter().filter(|(b, _, _)| b == bundle).map(|(_, n, _)| *n).collect();
        let mut slot = 0;
        while taken.contains(&slot) {
            slot += 1;
        }
        if taken.iter().any(|&n| n > slot) {
            return Err(ObsidianError::ForeignPatchGap { bundle: bundle.clone(), missing: slot });
        }
        plan.push((bundle, base, contents.join(format!("{bundle}.patch_{slot}"))));
    }
    let backup = backup_game_saves(game, &data_dir.join("backups"), opts.keep_backups)?;

    let mut temps = Vec::new();
    for (bundle, base, target) in &plan {
        let (b, stream) = patch_of(bundle::read_props(base)?, &comp.bundles[*bundle]);
        let tmp = target.with_extension(format!("{}.{TMP}", target.extension().unwrap().to_string_lossy()));
        temps.push((tmp.clone(), target.clone()));
        let mut written = bundle::write_file(&tmp, &b);
        if written.is_ok() && !stream.is_empty() {
            // The engine reads a patch's streamed bytes from `<patch>.stream`, as it does for a bundle.
            let target = beside(target, "stream");
            let tmp = beside(&target, TMP);
            temps.push((tmp.clone(), target));
            written = std::fs::write(&tmp, &stream).map_err(Into::into);
        }
        if let Err(e) = written {
            remove_all(temps.iter().map(|(t, _)| t));
            return Err(e);
        }
    }
    let files = temps
        .iter()
        .map(|(tmp, p)| Ok(DeployedFile { path: p.clone(), sha256: sha256_file(tmp)? }))
        .collect::<Result<Vec<_>>>()?;
    let mods = profile.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect();
    let state = DeployState { build, files, mods, time: chrono::Local::now().to_rfc3339(), fingerprint: Some(fingerprint), incomplete: false, previous: None };

    // Only now, with the new set ready, does the game change. The old patches step aside instead of
    // going, so a failure can put them back, and the state says what's under way first, so a crash
    // or a killed app is put right by the next deploy or purge.
    write_state(data_dir, &DeployState { incomplete: true, previous: previous.clone().map(Box::new), ..state.clone() })?;
    let mut aside: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut done: Vec<&PathBuf> = Vec::new();
    let swapped = (|| -> std::result::Result<(), (PathBuf, String)> {
        for f in previous.iter().flat_map(|p| &p.files) {
            let place = here(&contents, f);
            // A patch another tool changed stops being Obsidian's, as in a purge.
            if !place.is_file() || sha256_file(&place).map_err(|e| (place.clone(), e.to_string()))? != f.sha256 {
                continue;
            }
            let old = beside(&place, OLD);
            rename_retrying(&place, &old).map_err(|e| (place.clone(), e.to_string()))?;
            aside.push((old, place));
        }
        for (tmp, target) in &temps {
            if target.symlink_metadata().is_ok() {
                return Err((target.clone(), "another file already has this name".to_string()));
            }
            rename_retrying(tmp, target).map_err(|e| (target.clone(), e.to_string()))?;
            done.push(target);
        }
        Ok(())
    })();
    if let Err((file, why)) = swapped {
        remove_all(done.iter().copied());
        for (old, place) in &aside {
            let _ = rename_retrying(old, place);
        }
        remove_all(temps.iter().map(|(t, _)| t));
        match &previous {
            Some(p) => write_state(data_dir, p)?,
            None => std::fs::remove_file(data_dir.join(STATE))?,
        }
        return Err(ObsidianError::DeployFailed(format!("{}: {why}", file.display())));
    }
    write_state(data_dir, &state)?;
    for (old, _) in &aside {
        let _ = std::fs::remove_file(old);
    }
    settings::write(&settings::game_side_dir(&game.saves_dir), &settings)?;
    let foreign_paths = foreign.into_iter().filter(|(b, _, _)| comp.bundles.contains_key(b)).map(|(_, _, p)| p).collect();
    Ok(DeployReport { patches: state.files.into_iter().map(|f| f.path).collect(), conflicts: comp.conflicts, foreign: foreign_paths, backup })
}

/// One bundle's patch, and the bytes of its `.stream` (empty when nothing is streamed). A bundle's
/// `.stream` holds each variant's streamed part in the order of the entries and their variants, with
/// nothing between them, so the patch is sorted first and its stream follows that order.
fn patch_of(props: [u8; 256], planned: &[crate::compose::Planned]) -> (Bundle, Vec<u8>) {
    let mut b = Bundle { props, entries: Vec::new() };
    let mut streams = std::collections::HashMap::new();
    for p in planned {
        let variants = if p.pieces.is_empty() {
            vec![Variant { lang: 0, data: p.data.clone(), stream_size: 0 }]
        } else {
            let mut pieces: Vec<_> = p.pieces.iter().collect();
            pieces.sort_by_key(|x| x.lang);
            streams.insert((p.type_hash, p.name_hash), pieces.iter().map(|x| x.stream.as_slice()).collect::<Vec<_>>());
            pieces.iter().map(|x| Variant { lang: x.lang, data: x.data.clone(), stream_size: x.stream.len() as u32 }).collect()
        };
        b.entries.push(Entry { type_hash: p.type_hash, name_hash: p.name_hash, flags: p.flags, variants });
    }
    b.sort_entries();
    let mut stream = Vec::new();
    for e in &b.entries {
        for part in streams.get(&(e.type_hash, e.name_hash)).into_iter().flatten() {
            stream.extend_from_slice(part);
        }
    }
    (b, stream)
}

/// Removes the patches the last deploy recorded. Never consults the build gate. A file that can't go
/// doesn't stop the others, and stays recorded for the next try.
pub fn purge(game: &GameInstall, data_dir: &Path) -> Result<PurgeReport> {
    let contents = game.contents();
    finish_interrupted(&contents, data_dir)?;
    clear_leftovers(&contents)?;
    let mut report = PurgeReport { removed: Vec::new(), kept_changed: Vec::new() };
    let Some(mut state) = load_state(data_dir)? else { return Ok(report) };
    let mut stuck = Vec::new();
    for f in std::mem::take(&mut state.files) {
        // Where it is now, or where it was when the game lived somewhere else.
        let Some(path) = [here(&contents, &f), f.path.clone()].into_iter().find(|p| p.is_file()) else { continue };
        match sha256_file(&path).map(|hash| hash == f.sha256) {
            Ok(true) => match std::fs::remove_file(&path) {
                Ok(()) => report.removed.push(path),
                Err(e) => stuck.push((f, e.to_string())),
            },
            Ok(false) => report.kept_changed.push(path),
            Err(e) => stuck.push((f, e.to_string())),
        }
    }
    if let Some((f, why)) = stuck.first() {
        let message = format!("{}: {why}", f.path.display());
        state.files = stuck.into_iter().map(|(f, _)| f).collect();
        write_state(data_dir, &state)?;
        return Err(ObsidianError::PurgeStuck(message));
    }
    std::fs::remove_file(data_dir.join(STATE))?;
    Ok(report)
}

/// The patches in the game folder that aren't Obsidian's, by file name.
pub fn foreign_files(game: &GameInstall, data_dir: &Path) -> Result<Vec<String>> {
    let state = load_state(data_dir)?;
    Ok(foreign_patches(&game.contents(), state.as_ref())?.into_iter().filter_map(|(_, _, p)| p.file_name().map(|n| n.to_string_lossy().into_owned())).collect())
}

pub fn status(game: &GameInstall, data_dir: &Path, known: &[KnownBuild]) -> Result<Status> {
    let deployed = load_state(data_dir)?;
    let foreign = foreign_patches(&game.contents(), deployed.as_ref())?.into_iter().map(|(_, _, p)| p).collect();
    Ok(Status { gate: check_build(&game.root, known)?, deployed, foreign, crash_culprit: crash_culprit(game) })
}

/// The loader's log of the last launch.
pub fn loader_log(game: &GameInstall) -> PathBuf {
    settings::game_side_dir(&game.saves_dir).join("logs").join("loader.log")
}

/// The mod the loader was starting when the game last stopped, read from the loader log. Cheap,
/// unlike `status`, which also hashes the game files.
pub fn crash_culprit(game: &GameInstall) -> Option<String> {
    std::fs::read_to_string(loader_log(game)).ok().and_then(|t| loader::crash_culprit(&t))
}

/// A launch that stopped without the game closing normally.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UncleanEnd {
    /// When the log was last written, in seconds: it tells one crash from the next.
    pub stamp: u64,
    /// What the loader wrote last.
    pub last: String,
}

pub fn unclean_end(game: &GameInstall) -> Option<UncleanEnd> {
    let log = loader_log(game);
    let last = loader::unclean_end(&std::fs::read_to_string(&log).ok()?)?;
    let stamp = std::fs::metadata(&log).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(UncleanEnd { stamp, last })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{self, Bundle, Entry, Variant};
    use crate::gate::fake_build;
    use crate::library::{Library, Profile};
    use crate::testutil::{put, spaced_tempdir};
    use crate::ObsidianError;
    use std::collections::BTreeMap;

    struct World {
        _t: tempfile::TempDir,
        game: GameInstall,
        data: PathBuf,
        opts: DeployOptions<'static>,
    }

    fn base(name: u64) -> Bundle {
        let v = Variant { lang: 0, data: b"vanilla".to_vec(), stream_size: 0 };
        Bundle { props: [9; 256], entries: vec![Entry { type_hash: 1, name_hash: name, flags: 0, variants: vec![v] }] }
    }

    /// Bundles `..aa` and `..bb`, one mod per bundle (`mod-aa`, `mod-bb`), both enabled.
    fn world() -> World {
        let t = spaced_tempdir();
        let game = GameInstall { root: t.path().join("game"), saves_dir: t.path().join("saves"), cloud_saves: Vec::new() };
        put(&game.saves_dir, "profile.sav", b"save");
        std::fs::create_dir_all(game.contents()).unwrap();
        bundle::write_file(&game.contents().join("00000000000000aa"), &base(1)).unwrap();
        bundle::write_file(&game.contents().join("00000000000000bb"), &base(2)).unwrap();
        bundle::write_file(&game.contents().join(crate::loader::BUNDLE), &base(3)).unwrap();
        let known = fake_build(&game.root, &["contents/00000000000000aa", "contents/00000000000000bb", "contents/9cf71054ea26dddf"]);
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        for b in ["aa", "bb"] {
            let d = t.path().join(format!("stage-{b}"));
            let json = format!(
                r##"{{"obsidian":1,"id":"mod-{b}","version":"1.0.0","name":{{"en":"{b}"}},
                   "resources":[{{"type":"#0000000000000001","name":"#00000000000000f0","bundle":"00000000000000{b}","data":"r.bin"}}]}}"##
            );
            put(&d, "obsidian.json", json.as_bytes());
            put(&d, "r.bin", b"modded");
            lib.install(&d).unwrap();
        }
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        p.set_enabled("mod-aa", true).unwrap();
        p.set_enabled("mod-bb", true).unwrap();
        p.save(&data).unwrap();
        World { _t: t, game, data, opts: DeployOptions { known_builds: known, keep_backups: 10, loader_dir: None } }
    }

    fn snapshot(dir: &Path) -> BTreeMap<String, String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_file())
            .map(|p| (p.file_name().unwrap().to_string_lossy().to_string(), crate::gate::sha256_file(&p).unwrap()))
            .collect()
    }

    fn patch(w: &World, name: &str) -> PathBuf {
        w.game.contents().join(name)
    }

    #[test]
    fn deploy_writes_one_patch_per_bundle_and_purge_restores() {
        let w = world();
        let before = snapshot(&w.game.contents());
        assert!(!has_deployed(&w.data));
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(has_deployed(&w.data));
        assert_eq!(r.patches.len(), 3, "two mod patches and the loader");
        let p = bundle::read_file(&patch(&w, "00000000000000aa.patch_0")).unwrap();
        assert_eq!(p.props, [9u8; 256]);
        assert_eq!(p.entries[0].variants[0].data, b"modded");
        assert!(r.backup.is_some());
        assert!(status(&w.game, &w.data, w.opts.known_builds).unwrap().deployed.is_some());
        purge(&w.game, &w.data).unwrap();
        assert_eq!(snapshot(&w.game.contents()), before);
        assert!(status(&w.game, &w.data, w.opts.known_builds).unwrap().deployed.is_none());
        assert!(!has_deployed(&w.data));
    }

    fn settings_text(w: &World) -> String {
        std::fs::read_to_string(crate::settings::game_side_dir(&w.game.saves_dir).join(crate::settings::FILE)).unwrap()
    }

    #[test]
    fn loader_ships_with_any_enabled_mod() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let b = bundle::read_file(&patch(&w, "9cf71054ea26dddf.patch_0")).unwrap();
        let names: Vec<u64> = b.entries.iter().map(|e| e.name_hash).collect();
        for n in [crate::loader::ENTRY, "obsidian/loader/core", "obsidian/loader/settings", "obsidian/deploy"] {
            assert!(names.contains(&crate::hash::hash_name(n)), "{n}");
        }
        assert!(settings_text(&w).contains("consent	0
"));
        purge(&w.game, &w.data).unwrap();
        assert!(!patch(&w, "9cf71054ea26dddf.patch_0").exists());
        assert!(settings_text(&w).contains("consent"), "purge leaves settings.txt");
    }

    #[test]
    fn consent_alone_deploys_the_loader() {
        let w = world();
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-aa", false).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.save(&w.data).unwrap();
        assert!(deploy(&w.game, &w.data, &w.opts).unwrap().patches.is_empty(), "nothing on, nothing written");
        p.consent = true;
        p.save(&w.data).unwrap();
        assert_eq!(deploy(&w.game, &w.data, &w.opts).unwrap().patches, vec![patch(&w, "9cf71054ea26dddf.patch_0")]);
        assert!(settings_text(&w).contains("consent	1
"));
    }

    #[test]
    fn slot_follows_foreign_patches() {
        let w = world();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_0"), &base(7)).unwrap();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_1"), &base(8)).unwrap();
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(r.patches.contains(&patch(&w, "00000000000000aa.patch_2")));
        assert_eq!(r.foreign.len(), 2);
        purge(&w.game, &w.data).unwrap();
        assert!(patch(&w, "00000000000000aa.patch_1").exists(), "foreign patches are never removed");
    }

    /// Turns on a mod for a third bundle, and blocks its patch with a folder, so the swap fails after
    /// the old patches moved and some new ones landed.
    fn block_the_swap(w: &World) {
        let d = w.data.join("stage-cc");
        let json = r##"{"obsidian":1,"id":"zz-cc","version":"1.0.0","name":{"en":"cc"},
            "resources":[{"type":"#0000000000000001","name":"#00000000000000f1","bundle":"00000000000000cc","data":"r.bin"}]}"##;
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "r.bin", b"modded");
        bundle::write_file(&w.game.contents().join("00000000000000cc"), &base(4)).unwrap();
        let lib = Library::open(&w.data).unwrap();
        lib.install(&d).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.sync(&lib).unwrap();
        p.set_enabled("zz-cc", true).unwrap();
        p.save(&w.data).unwrap();
        std::fs::create_dir_all(patch(w, "00000000000000cc.patch_0")).unwrap();
    }

    #[test]
    fn a_failed_swap_puts_the_last_deploy_back() {
        let mut w = world();
        // The third bundle has to pass the build check too.
        w.opts.known_builds = fake_build(&w.game.root, &["contents/00000000000000aa", "contents/00000000000000bb", "contents/9cf71054ea26dddf"]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let (files, state) = (snapshot(&w.game.contents()), load_state(&w.data).unwrap());
        block_the_swap(&w);
        let before = snapshot(&w.game.contents());
        let e = deploy(&w.game, &w.data, &w.opts).unwrap_err();
        assert!(matches!(e, ObsidianError::DeployFailed(_)), "{e}");
        assert_eq!(snapshot(&w.game.contents()), before, "every patch is back, and nothing is left over");
        assert!(before.keys().all(|k| files.contains_key(k) || k == "00000000000000cc"));
        assert_eq!(load_state(&w.data).unwrap(), state, "the record is back too");
    }

    #[test]
    fn a_deploy_cut_short_counts_as_waiting_and_is_put_right() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let before = snapshot(&w.game.contents());
        let done = load_state(&w.data).unwrap().unwrap();
        // As a kill would leave it: the state says a swap is under way, and an old patch is aside.
        let cut = DeployState { incomplete: true, previous: Some(Box::new(done.clone())), files: Vec::new(), ..done.clone() };
        write_state(&w.data, &cut).unwrap();
        let moved = patch(&w, "00000000000000aa.patch_0");
        std::fs::rename(&moved, beside(&moved, OLD)).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);

        finish_interrupted(&w.game.contents(), &w.data).unwrap();
        assert_eq!(snapshot(&w.game.contents()), before);
        assert_eq!(load_state(&w.data).unwrap().unwrap(), done);
    }

    #[test]
    fn purge_after_a_cut_short_deploy_leaves_the_game_vanilla() {
        let w = world();
        let vanilla = snapshot(&w.game.contents());
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let done = load_state(&w.data).unwrap().unwrap();
        write_state(&w.data, &DeployState { incomplete: true, previous: Some(Box::new(done.clone())), files: Vec::new(), ..done }).unwrap();
        let moved = patch(&w, "00000000000000bb.patch_0");
        std::fs::rename(&moved, beside(&moved, OLD)).unwrap();
        purge(&w.game, &w.data).unwrap();
        assert_eq!(snapshot(&w.game.contents()), vanilla);
    }

    /// The state as written with the game's path spelled another way.
    fn respell(w: &World, f: impl Fn(&str) -> String) {
        let mut s = load_state(&w.data).unwrap().unwrap();
        for file in &mut s.files {
            file.path = PathBuf::from(f(&file.path.to_string_lossy()));
        }
        write_state(&w.data, &s).unwrap();
    }

    #[test]
    fn own_patches_are_known_however_the_path_is_spelled() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        respell(&w, |p| p.replace("game", "GAME-with-another-name"));
        assert_eq!(status(&w.game, &w.data, w.opts.known_builds).unwrap().foreign, Vec::<PathBuf>::new());
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(r.patches.iter().all(|p| p.to_string_lossy().ends_with(".patch_0")), "{:?}", r.patches);
    }

    #[test]
    fn purge_finds_patches_after_the_game_moved() {
        let w = world();
        let vanilla = snapshot(&w.game.contents());
        deploy(&w.game, &w.data, &w.opts).unwrap();
        respell(&w, |p| p.replace("game", "old-drive-game"));
        let r = purge(&w.game, &w.data).unwrap();
        assert_eq!(r.removed.len(), 3);
        assert_eq!(snapshot(&w.game.contents()), vanilla);
    }

    #[test]
    fn patches_that_went_missing_count_as_waiting() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
        std::fs::remove_file(patch(&w, "00000000000000aa.patch_0")).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
    }

    #[test]
    fn foreign_gap_is_refused() {
        let w = world();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_0"), &base(7)).unwrap();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_2"), &base(8)).unwrap();
        assert!(matches!(deploy(&w.game, &w.data, &w.opts), Err(ObsidianError::ForeignPatchGap { missing: 1, .. })));
        assert!(!patch(&w, "00000000000000bb.patch_0").exists());
    }

    #[test]
    fn redeploy_replaces_previous_patches() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.save(&w.data).unwrap();
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(r.patches, vec![patch(&w, "00000000000000aa.patch_0"), patch(&w, "9cf71054ea26dddf.patch_0")]);
        assert!(!patch(&w, "00000000000000bb.patch_0").exists());
    }

    #[test]
    fn deploy_cleans_leftover_tmp_files() {
        let w = world();
        put(&w.game.contents(), "00000000000000aa.patch_0.obsidian-tmp", b"junk");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(!patch(&w, "00000000000000aa.patch_0.obsidian-tmp").exists());
    }

    #[test]
    fn deploy_rolls_back_when_a_rename_fails() {
        let w = world();
        put(&patch(&w, "00000000000000bb.patch_0"), "blocker", b"x");
        assert!(matches!(deploy(&w.game, &w.data, &w.opts), Err(ObsidianError::DeployFailed(_))));
        assert!(!patch(&w, "00000000000000aa.patch_0").exists());
        assert!(!patch(&w, "00000000000000aa.patch_0.obsidian-tmp").exists());
        assert!(!w.data.join(STATE).exists());
    }

    #[test]
    fn gate_failure_writes_nothing() {
        let w = world();
        bundle::write_file(&w.game.contents().join("00000000000000aa"), &base(5)).unwrap();
        let before = snapshot(&w.game.contents());
        assert!(matches!(deploy(&w.game, &w.data, &w.opts), Err(ObsidianError::UnsupportedBuild(_))));
        assert_eq!(snapshot(&w.game.contents()), before);
        assert!(!w.data.join("backups").exists());
    }

    #[test]
    fn purge_works_when_gate_fails() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        bundle::write_file(&w.game.contents().join("00000000000000aa"), &base(5)).unwrap();
        assert_eq!(purge(&w.game, &w.data).unwrap().removed.len(), 3);
    }

    #[test]
    fn failed_redeploy_keeps_the_previous_deploy() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        std::fs::remove_file(w.data.join("mods/mod-bb/r.bin")).unwrap();
        assert!(deploy(&w.game, &w.data, &w.opts).is_err());
        assert!(patch(&w, "00000000000000aa.patch_0").exists() && patch(&w, "00000000000000bb.patch_0").exists());
        assert!(w.data.join(STATE).exists());
    }

    #[test]
    fn foreign_patch_in_upper_case_is_respected() {
        let w = world();
        bundle::write_file(&patch(&w, "00000000000000AA.patch_0"), &base(7)).unwrap();
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(r.patches.contains(&patch(&w, "00000000000000aa.patch_1")));
        purge(&w.game, &w.data).unwrap();
        assert_eq!(bundle::read_file(&patch(&w, "00000000000000AA.patch_0")).unwrap(), base(7));
    }

    #[test]
    fn state_is_written_atomically_and_removed_on_rollback() {
        let w = world();
        put(&w.data, "deploy-state.json.obsidian-tmp", b"torn");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(!w.data.join("deploy-state.json.obsidian-tmp").exists());
        purge(&w.game, &w.data).unwrap();
        put(&patch(&w, "00000000000000bb.patch_0"), "blocker", b"x");
        assert!(deploy(&w.game, &w.data, &w.opts).is_err());
        assert!(!w.data.join(STATE).exists());
    }

    /// Adds client mod `opt` (on, last): an entry, a live toggle `fast`, a deploy choice `sky`, and
    /// the resource `mod-aa` changes too.
    fn add_option_mod(w: &World) {
        let d = w.data.join("..").join("stage-opt");
        put(&d, "obsidian.json", br##"{"obsidian":1,"id":"opt","version":"1.0.0","name":{"en":"o"},"entry":"main.lua","scope":"client",
            "options":[{"id":"fast","type":"toggle","default":false},{"id":"sky","type":"choice","default":"day","choices":["day","night"]}],
            "resources":[
              {"type":"#0000000000000001","name":"#00000000000000f0","bundle":"00000000000000aa","data":"day.bin","when":{"sky":"day"}},
              {"type":"#0000000000000001","name":"#00000000000000f0","bundle":"00000000000000aa","data":"night.bin","when":{"sky":"night"}}]}"##);
        put(&d, "main.lua", b"return true\n");
        put(&d, "day.bin", b"day");
        put(&d, "night.bin", b"night");
        let lib = Library::open(&w.data).unwrap();
        lib.install(&d).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.sync(&lib).unwrap();
        p.set_enabled("opt", true).unwrap();
        p.save(&w.data).unwrap();
    }

    fn waiting(w: &World) -> ChangesWaiting {
        let lib = Library::open(&w.data).unwrap();
        changes_waiting(&lib, &Profile::load(&w.data).unwrap(), &w.data, Some(&w.game.contents())).unwrap()
    }

    fn change(w: &World, f: impl FnOnce(&mut Profile)) {
        let mut p = Profile::load(&w.data).unwrap();
        f(&mut p);
        p.save(&w.data).unwrap();
    }

    #[test]
    fn fresh_deploy_is_up_to_date() {
        let w = world();
        add_option_mod(&w);
        assert_eq!(waiting(&w), ChangesWaiting::Waiting, "never deployed");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
    }

    #[test]
    fn toggling_a_mod_is_waiting() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        change(&w, |p| p.set_enabled("mod-bb", false).unwrap());
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
        change(&w, |p| p.set_enabled("mod-bb", true).unwrap());
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "back as deployed");
    }

    #[test]
    fn changing_a_live_option_is_not_waiting() {
        let w = world();
        add_option_mod(&w);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        change(&w, |p| {
            p.options.entry("opt".into()).or_default().insert("fast".into(), serde_json::json!(true));
            p.keys.entry("opt".into()).or_default().insert("toggle".into(), "f9".into());
        });
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
    }

    #[test]
    fn changing_a_deploy_option_is_waiting() {
        let w = world();
        add_option_mod(&w);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        change(&w, |p| {
            p.options.entry("opt".into()).or_default().insert("sky".into(), serde_json::json!("night"));
        });
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
    }

    #[test]
    fn reordering_conflicting_mods_is_waiting() {
        let w = world();
        add_option_mod(&w);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        change(&w, |p| p.set_order(&["opt".to_string()]));
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
    }

    #[test]
    fn nothing_on_and_nothing_deployed_is_vanilla() {
        let w = world();
        change(&w, |p| {
            p.set_enabled("mod-aa", false).unwrap();
            p.set_enabled("mod-bb", false).unwrap();
        });
        assert_eq!(waiting(&w), ChangesWaiting::Vanilla);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Vanilla, "a deploy that wrote nothing");
        change(&w, |p| p.consent = true);
        assert_eq!(waiting(&w), ChangesWaiting::Waiting, "consent alone needs the loader");
    }

    #[test]
    fn old_state_without_fingerprint_is_waiting() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let path = w.data.join(STATE);
        let mut state: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        state.as_object_mut().unwrap().remove("fingerprint");
        std::fs::write(&path, state.to_string()).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
    }

    #[test]
    fn purge_keeps_patches_changed_by_someone_else() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_0"), &base(6)).unwrap();
        let r = purge(&w.game, &w.data).unwrap();
        assert_eq!(r.kept_changed, vec![patch(&w, "00000000000000aa.patch_0")]);
        assert!(patch(&w, "00000000000000aa.patch_0").exists());
    }

    fn piece(lang: u32, data: &[u8], stream: &[u8]) -> crate::compose::Piece {
        crate::compose::Piece { lang, data: data.to_vec(), stream: stream.to_vec() }
    }

    #[test]
    fn a_patch_stream_follows_its_entries_and_their_languages() {
        use crate::compose::Planned;
        // Given out of order: the patch sorts entries by (type, name) and variants by language, and the
        // stream follows that order, as a game bundle's does.
        let planned = vec![
            Planned::with_pieces(5, 2, 0, vec![piece(0x100, b"b-fr", b"FR"), piece(0, b"b", b"BBB")], "m".into()),
            Planned::new(5, 1, 0, b"plain".to_vec(), "m".into()),
            Planned::with_pieces(4, 9, 0, vec![piece(0, b"a", b"A")], "m".into()),
        ];
        let (b, stream) = patch_of([0; 256], &planned);
        let order: Vec<_> = b.entries.iter().map(|e| (e.type_hash, e.name_hash)).collect();
        assert_eq!(order, [(4, 9), (5, 1), (5, 2)]);
        assert_eq!(stream, b"ABBBFR");
        let langs: Vec<_> = b.entries[2].variants.iter().map(|v| (v.lang, v.stream_size)).collect();
        assert_eq!(langs, [(0, 3), (0x100, 2)]);
        assert_eq!(b.entries[1].variants, vec![Variant { lang: 0, data: b"plain".to_vec(), stream_size: 0 }]);
        assert!(patch_of([0; 256], &planned[1..2]).1.is_empty(), "no stream without streamed bytes");
    }

    #[test]
    fn a_streamed_bank_deploys_with_its_stream_file_and_purge_takes_both() {
        let w = world();
        let before = snapshot(&w.game.contents());
        let d = w._t.path().join("stage-audio");
        let json = r##"{"obsidian":1,"id":"mod-audio","version":"1.0.0","name":{"en":"audio"},"scope":"client",
            "resources":[{"type":"timpani_bank","name":"#00000000000000f1","bundle":"00000000000000aa",
              "variants":[{"data":"en.bin","stream":"en.stream"},{"lang":256,"data":"fr.bin","stream":"fr.stream"}]}]}"##;
        put(&d, "obsidian.json", json.as_bytes());
        for (f, b) in [("en.bin", "E"), ("en.stream", "en-stream"), ("fr.bin", "F"), ("fr.stream", "fr-stream")] {
            put(&d, f, b.as_bytes());
        }
        let lib = Library::open(&w.data).unwrap();
        lib.install(&d).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.sync(&lib).unwrap();
        p.set_enabled("mod-audio", true).unwrap();
        p.save(&w.data).unwrap();

        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        let stream = patch(&w, "00000000000000aa.patch_0.stream");
        assert!(r.patches.contains(&stream));
        assert_eq!(std::fs::read(&stream).unwrap(), b"en-streamfr-stream");
        let b = bundle::read_file(&patch(&w, "00000000000000aa.patch_0")).unwrap();
        let bank = b.entries.iter().find(|e| e.name_hash == 0xf1).unwrap();
        assert_eq!(bank.variants.iter().map(|v| (v.lang, v.stream_size)).collect::<Vec<_>>(), [(0, 9), (256, 9)]);
        assert!(!patch(&w, "00000000000000bb.patch_0.stream").exists(), "a bundle with nothing streamed gets no stream file");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "plan (digests from stamps) agrees with what deploy wrote");
        purge(&w.game, &w.data).unwrap();
        assert_eq!(snapshot(&w.game.contents()), before);
    }
}
