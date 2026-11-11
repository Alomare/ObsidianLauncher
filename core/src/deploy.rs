//! Deploy writes Obsidian's patches next to the game's bundles; purge removes exactly those.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backup::backup_game_saves;
use crate::bundle::{self, Bundle, Entry, Variant};
use crate::compose::{Composition, Conflict};
use crate::game::GameInstall;
use crate::gate::{GateReport, KNOWN_BUILDS, KnownBuild, check_build, sha256_file, sha256_files};
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
    /// `settings_digest` of the option values and keys the deploy wrote; missing in older states.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
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
    /// Told what the deploy is doing, from the start to the end of every deploy.
    pub progress: Option<&'a (dyn Fn(&DeployProgress) + Sync)>,
    /// Puts the loader in without touching the mods: the set the last deploy put in the game stays as
    /// it was, whatever the profile says now, and a game with no mods gets the loader alone. The
    /// loader goes in when something needs it (a selected Sandbox save, or a guardrail). The Saves tab's deploy.
    pub loader_only: bool,
}

/// What a deploy is doing right now.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeployProgress {
    /// Of the whole deploy, 0 to 100.
    pub percent: u32,
    /// The display name of the mod being worked on; none for a shared step.
    #[serde(rename = "mod")]
    pub mod_name: Option<String>,
    /// `preparing`, `resources`, `scripts`, `sound banks`, `writing patches` or `done`.
    pub stage: &'static str,
}

/// Sends progress, leaving out a report the same as the last one: a patch writes many blocks.
struct Reporter<'a> {
    sink: Option<&'a (dyn Fn(&DeployProgress) + Sync)>,
    last: std::sync::Mutex<Option<DeployProgress>>,
}

impl Reporter<'_> {
    fn send(&self, part: f32, stage: &'static str, mod_name: Option<String>) {
        let Some(sink) = self.sink else { return };
        let now = DeployProgress { percent: (part.clamp(0.0, 1.0) * 100.0) as u32, mod_name, stage };
        let mut last = self.last.lock().unwrap_or_else(|p| p.into_inner());
        if last.as_ref() != Some(&now) {
            sink(&now);
            *last = Some(now);
        }
    }
}

/// How much of the bar reading the mods and compiling the scripts take; the patches get the rest.
const READING_END: f32 = 0.3;
const SCRIPTS_END: f32 = 0.4;

fn display_of(name: &crate::manifest::Localized) -> Option<String> {
    name.get("en").or_else(|| name.values().next()).cloned()
}

impl Default for DeployOptions<'static> {
    fn default() -> Self {
        DeployOptions { known_builds: KNOWN_BUILDS, keep_backups: 10, loader_dir: None, progress: None, loader_only: false }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeployReport {
    pub patches: Vec<PathBuf>,
    pub conflicts: Vec<Conflict>,
    pub foreign: Vec<PathBuf>,
    pub backup: Option<PathBuf>,
    /// The game already had this deploy, so no patch was written (only the live settings).
    pub unchanged: bool,
    /// The option values or keys differ from the last deploy.
    pub settings_changed: bool,
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
/// `changed`: recorded patches another tool has changed since, which count as that tool's, as in a purge.
fn foreign_patches(contents: &Path, state: Option<&DeployState>, changed: &[PathBuf]) -> Result<Vec<(String, u32, PathBuf)>> {
    let recorded = state.into_iter().flat_map(|s| s.files.iter().chain(s.previous.iter().flat_map(|p| &p.files)));
    let ours: Vec<String> = recorded
        .filter(|f| !changed.contains(&here(contents, f)))
        .filter_map(|f| f.path.file_name())
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .collect();
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

/// Whether the last deploy put the loader in the game, and the patch that holds it is still there.
pub fn loader_deployed(game: &GameInstall, data_dir: &Path) -> bool {
    let contents = game.contents();
    load_state(data_dir).ok().flatten().is_some_and(|s| !s.incomplete && s.files.iter().any(|f| f.path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(loader::BUNDLE)) && here(&contents, f).is_file()))
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

/// One hash of the settings file a deploy writes: option values, keys and the guardrails.
pub fn settings_digest(s: &settings::Settings) -> String {
    use sha2::{Digest, Sha256};
    // Selecting a save reaches the game through the settings file and asks for no deploy.
    let s = settings::Settings { sandbox: None, ..s.clone() };
    Sha256::digest(settings::render(&s).as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// `contents` is the game's folder of bundles, to check that the recorded patches are still there.
pub fn changes_waiting(lib: &Library, profile: &Profile, data_dir: &Path, contents: Option<&Path>) -> Result<ChangesWaiting> {
    let mut profile = profile.clone();
    profile.sync(lib)?;
    let comp = crate::compose::plan_for(lib, &profile, contents)?;
    let digest = settings_digest(&loader::settings_for(lib, &profile)?);
    changes_waiting_for(&comp, &profile, &digest, data_dir, contents)
}

/// `changes_waiting` for a composition already made from `profile`, synced with the library. A deploy
/// that was cut short, or patches that are gone (a reinstall, another copy of the game), count as
/// changes waiting: Play would otherwise start the game without the mods. So do option values and
/// keys changed since the deploy (`settings`, from `settings_digest`), although they reach the
/// game at the next launch anyway: Deploy says the game has what the profile shows.
pub fn changes_waiting_for(comp: &Composition, profile: &Profile, settings: &str, data_dir: &Path, contents: Option<&Path>) -> Result<ChangesWaiting> {
    let (comp, profile) = (comp, profile);
    let state = load_state(data_dir)?;
    let deployed_nothing = state.as_ref().is_none_or(|s| s.files.is_empty());
    if !comp.unplaced.is_empty() {
        return Ok(ChangesWaiting::Waiting);
    }
    if comp.bundles.is_empty() && !loader::needed(comp, profile) {
        return Ok(if deployed_nothing { ChangesWaiting::Vanilla } else { ChangesWaiting::Waiting });
    }
    let now = fingerprint(comp, profile, &loader::find_loader_dir()?)?;
    let whole = state.as_ref().is_some_and(|s| !s.incomplete && contents.is_none_or(|c| s.files.iter().all(|f| here(c, f).is_file())));
    // A state from before the settings were recorded can't tell, so one deploy (a quick one) records them.
    let same_settings = state.as_ref().is_some_and(|s| s.settings.as_deref() == Some(settings));
    Ok(match state.and_then(|s| s.fingerprint) {
        Some(f) if f == now && whole && same_settings => ChangesWaiting::UpToDate,
        _ => ChangesWaiting::Waiting,
    })
}

/// One deploy or purge at a time per library, whether from the app or the command line: each starts by
/// clearing half-written files, which would be the other one's. Held until the returned file drops.
fn hold_lock(data_dir: &Path) -> Result<std::fs::File> {
    std::fs::create_dir_all(data_dir)?;
    let f = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(data_dir.join("deploy.lock"))?;
    // On Linux a child process started by another thread holds a copy of the lock until the child
    // starts the program, so a lock dropped a moment ago can still look held. A short wait covers that.
    for _ in 0..25 {
        match f.try_lock() {
            Ok(()) => return Ok(f),
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(std::fs::TryLockError::Error(e)) => return Err(e.into()),
        }
    }
    Err(ObsidianError::Busy)
}

pub fn deploy(game: &GameInstall, data_dir: &Path, opts: &DeployOptions) -> Result<DeployReport> {
    let _lock = hold_lock(data_dir)?;
    let report = Reporter { sink: opts.progress, last: Default::default() };
    report.send(0.0, "preparing", None);
    let contents = game.contents();
    let gate = check_build(&game.root, opts.known_builds)?;
    // A build Obsidian doesn't know still gets the deploy: the patches never touch the game's own
    // files, and the loader keeps every mod off at boot when the game's version isn't the one it
    // was made for. Mods copied over the game's files are the usual cause, and the app says so.
    let build = gate.build.clone().unwrap_or_else(|| format!("unknown, differs in {}", gate.mismatches.join(", ")));
    finish_interrupted(&contents, data_dir)?;
    clear_leftovers(&contents)?;
    let previous = load_state(data_dir)?;
    let lib = Library::open(data_dir)?;
    let mut profile = Profile::load(data_dir)?;
    profile.sync(&lib)?;
    if opts.loader_only {
        // Only this deploy's copy: the profile on disk is left as it is, and the Mods tab shows any change waiting.
        let kept: &[String] = previous.as_ref().filter(|p| !p.incomplete).map_or(&[], |p| &p.mods);
        for e in &mut profile.mods {
            e.enabled = kept.contains(&e.id);
        }
    }
    let loader_dir = match &opts.loader_dir {
        Some(d) => d.clone(),
        None => loader::find_loader_dir()?,
    };
    // The game may have this deploy already: the same fingerprint, and every patch where it was and as
    // it was written. Then nothing is rebuilt, which with sound banks saves half a minute; live options
    // aren't in the patches, so the settings are written all the same.
    if let Some(prev) = previous.as_ref().filter(|p| !p.incomplete && !p.files.is_empty()) {
        let planned = crate::compose::plan_for(&lib, &profile, Some(&contents))?;
        let intact = || {
            let places: Vec<PathBuf> = prev.files.iter().map(|f| here(&contents, f)).collect();
            sha256_files(&places).into_iter().zip(&prev.files).all(|(now, f)| now.is_ok_and(|s| s == f.sha256))
        };
        if planned.unplaced.is_empty() && prev.fingerprint.as_deref() == Some(fingerprint(&planned, &profile, &loader_dir)?.as_str()) && intact() {
            let live = loader::settings_for(&lib, &profile)?;
            let digest = settings_digest(&live);
            settings::write(&settings::game_side_dir(&game.saves_dir), &live)?;
            let settings_changed = prev.settings.as_deref() != Some(digest.as_str());
            if settings_changed {
                write_state(data_dir, &DeployState { settings: Some(digest), ..prev.clone() })?;
            }
            let foreign = foreign_patches(&contents, previous.as_ref(), &[])?;
            let foreign = foreign.into_iter().filter(|(b, _, _)| planned.bundles.contains_key(b)).map(|(_, _, p)| p).collect();
            let patches = prev.files.iter().map(|f| here(&contents, f)).collect();
            report.send(1.0, "done", None);
            return Ok(DeployReport { patches, conflicts: planned.conflicts, foreign, backup: None, unchanged: true, settings_changed });
        }
    }
    let on_mod = |i: usize, total: usize, m: &crate::manifest::Manifest| {
        report.send(READING_END * i as f32 / total.max(1) as f32, "resources", display_of(&m.name));
    };
    let mut comp = crate::compose::compose_reporting(&lib, &profile, Some(&contents), &on_mod)?;
    let fingerprint = fingerprint(&comp, &profile, &loader_dir)?;
    if loader::needed(&comp, &profile) {
        report.send(READING_END, "scripts", None);
        let dir = &loader_dir;
        let planned = loader::build(dir, &comp)?;
        let slot = comp.bundles.entry(loader::BUNDLE.to_string()).or_default();
        for p in planned {
            slot.retain(|q| (q.type_hash, q.name_hash) != (p.type_hash, p.name_hash));
            slot.push(p);
        }
    }
    let settings = loader::settings_for(&lib, &profile)?;
    let digest = settings_digest(&settings);
    let settings_changed = previous.as_ref().is_none_or(|p| p.settings.as_deref() != Some(digest.as_str()));

    // Plan every slot before changing anything, so a refusal leaves the previous deploy in place. A
    // patch of the last deploy that another tool has changed stops being Obsidian's, as in a purge: it
    // stays, and keeps its slot.
    let mut changed = Vec::new();
    for f in previous.iter().flat_map(|p| &p.files) {
        let place = here(&contents, f);
        if place.is_file() && sha256_file(&place)? != f.sha256 {
            changed.push(place);
        }
    }
    let foreign = foreign_patches(&contents, previous.as_ref(), &changed)?;
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
        // The game reads patches from 0 up and stops at a missing number: Obsidian's fills one hole, and
        // a hole left after that hides what is above it. A patch dropped in by hand above Obsidian's,
        // as some mods' readmes say to do, stays there and wins over Obsidian's.
        let last = taken.iter().copied().max().unwrap_or(0);
        if let Some(missing) = (0..=last).find(|n| !taken.contains(n) && *n != slot) {
            return Err(ObsidianError::ForeignPatchGap { bundle: bundle.clone(), missing });
        }
        plan.push((bundle, base, contents.join(format!("{bundle}.patch_{slot}"))));
    }
    let backup = backup_game_saves(game, &data_dir.join("backups"), opts.keep_backups)?;

    let mut temps = Vec::new();
    let count = plan.len().max(1) as f32;
    let name_of = |owner: &str| comp.mods.iter().find(|m| m.manifest.id == owner).and_then(|m| display_of(&m.manifest.name));
    for (i, (bundle, base, target)) in plan.iter().enumerate() {
        let planned = &comp.bundles[*bundle];
        let banks = planned.iter().filter(|p| p.recipe.is_some()).count();
        // Rebuilding the banks of a bundle takes about as long as writing the patch.
        let writing_from = if banks > 0 { 0.5 } else { 0.0 };
        let share = |part: f32| SCRIPTS_END + (1.0 - SCRIPTS_END) * (i as f32 + part.min(1.0)) / count;
        // A patch holding one mod's files is that mod's; a shared one is nobody's in particular.
        let owner = planned.first().filter(|f| planned.iter().all(|p| p.owner == f.owner)).and_then(|f| name_of(&f.owner));
        report.send(share(0.0), if banks > 0 { "sound banks" } else { "writing patches" }, if banks > 0 { None } else { owner.clone() });
        let mut rebuilt_banks = 0;
        // Sound banks are rebuilt here, one bundle at a time, as a big one can pass a gigabyte.
        let built = planned
            .iter()
            .cloned()
            .map(|p| {
                if p.recipe.is_some() {
                    report.send(share(writing_from * rebuilt_banks as f32 / banks as f32), "sound banks", name_of(&p.owner));
                    rebuilt_banks += 1;
                }
                crate::compose::rebuilt(&contents, bundle, p)
            })
            .collect::<Result<Vec<_>>>()
            .and_then(|planned| Ok(patch_of(bundle::read_props(base)?, planned)));
        let (b, stream) = match built {
            Ok(built) => built,
            Err(e) => {
                remove_all(temps.iter().map(|(t, _)| t));
                return Err(e);
            }
        };
        let tmp = target.with_extension(format!("{}.{TMP}", target.extension().unwrap().to_string_lossy()));
        temps.push((tmp.clone(), target.clone()));
        let mut written = bundle::write_file_with(&tmp, &b, &mut |part| report.send(share(writing_from + (1.0 - writing_from) * part), "writing patches", owner.clone()));
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
    let written: Vec<PathBuf> = temps.iter().map(|(tmp, _)| tmp.clone()).collect();
    let files = sha256_files(&written)
        .into_iter()
        .zip(&temps)
        .map(|(sha256, (_, p))| Ok(DeployedFile { path: p.clone(), sha256: sha256? }))
        .collect::<Result<Vec<_>>>()?;
    let mods = profile.mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect();
    let state = DeployState { build, files, mods, time: chrono::Local::now().to_rfc3339(), fingerprint: Some(fingerprint), settings: Some(digest), incomplete: false, previous: None };

    // Only now, with the new set ready, does the game change. The old patches step aside instead of
    // going, so a failure can put them back, and the state says what's under way first, so a crash
    // or a killed app is put right by the next deploy or purge.
    write_state(data_dir, &DeployState { incomplete: true, previous: previous.clone().map(Box::new), ..state.clone() })?;
    let mut aside: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut done: Vec<&PathBuf> = Vec::new();
    let swapped = (|| -> std::result::Result<(), (PathBuf, String)> {
        for f in previous.iter().flat_map(|p| &p.files) {
            let place = here(&contents, f);
            if !place.is_file() || changed.contains(&place) {
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
    let game_side = settings::game_side_dir(&game.saves_dir);
    settings::settle_carry_over(&game_side);
    settings::write(&game_side, &settings)?;
    report.send(1.0, "done", None);
    let foreign_paths = foreign.into_iter().filter(|(b, _, _)| comp.bundles.contains_key(b)).map(|(_, _, p)| p).collect();
    Ok(DeployReport { patches: state.files.into_iter().map(|f| f.path).collect(), conflicts: comp.conflicts, foreign: foreign_paths, backup, unchanged: false, settings_changed })
}

/// One bundle's patch, and the bytes of its `.stream` (empty when nothing is streamed). A bundle's
/// `.stream` holds each variant's streamed part in the order of the entries and their variants, with
/// nothing between them, so the patch is sorted first and its stream follows that order.
fn patch_of(props: [u8; 256], planned: Vec<crate::compose::Planned>) -> (Bundle, Vec<u8>) {
    let mut b = Bundle { props, entries: Vec::new() };
    let mut streams = std::collections::HashMap::new();
    for mut p in planned {
        let variants = if p.pieces.is_empty() {
            vec![Variant { lang: 0, data: p.data, stream_size: 0 }]
        } else {
            p.pieces.sort_by_key(|x| x.lang);
            let mut parts = Vec::with_capacity(p.pieces.len());
            let mut variants = Vec::with_capacity(p.pieces.len());
            for x in p.pieces {
                variants.push(Variant { lang: x.lang, stream_size: x.stream.len() as u32, data: x.data });
                parts.push(x.stream);
            }
            streams.insert((p.type_hash, p.name_hash), parts);
            variants
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
    let _lock = hold_lock(data_dir)?;
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

/// A patch in the game folder that isn't Obsidian's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForeignPatch {
    pub name: String,
    /// The game bundle it patches, as 16 hex digits.
    pub bundle: String,
    /// Bytes, with its `.stream` when there is one.
    pub size: u64,
    /// When it was last written, in seconds since 1970.
    pub modified: Option<u64>,
}

/// The patches in the game folder that aren't Obsidian's.
pub fn foreign_files(game: &GameInstall, data_dir: &Path) -> Result<Vec<ForeignPatch>> {
    let state = load_state(data_dir)?;
    let mut out = Vec::new();
    for (bundle, _, p) in foreign_patches(&game.contents(), state.as_ref(), &[])? {
        let meta = std::fs::metadata(&p)?;
        let stream = std::fs::metadata(beside(&p, "stream")).map(|m| m.len()).unwrap_or(0);
        let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
        out.push(ForeignPatch { name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), bundle, size: meta.len() + stream, modified });
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForeignRemoved {
    /// The folder that holds a copy of every patch taken out.
    pub kept_in: Option<PathBuf>,
    pub removed: Vec<PathBuf>,
    /// Obsidian's patches moved down to fill the numbers the removed ones leave free.
    pub renumbered: usize,
}

/// Moves file `from` to `to`, across drives too.
fn move_file(from: &Path, to: &Path) -> Result<()> {
    if rename_retrying(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)?;
    Ok(())
}

/// Takes every patch that isn't Obsidian's out of the game folder, keeping a copy of each (and of
/// its `.stream`) in `keep_dir`. The game reads a bundle's patches from 0 up and stops at the first
/// number missing, so Obsidian's own patches then move down to fill the gaps, and the state follows.
pub fn remove_foreign(game: &GameInstall, data_dir: &Path, keep_dir: &Path) -> Result<ForeignRemoved> {
    let _lock = hold_lock(data_dir)?;
    let contents = game.contents();
    finish_interrupted(&contents, data_dir)?;
    clear_leftovers(&contents)?;
    let mut state = load_state(data_dir)?;
    let foreign = foreign_patches(&contents, state.as_ref(), &[])?;
    let mut out = ForeignRemoved { kept_in: None, removed: Vec::new(), renumbered: 0 };
    if foreign.is_empty() {
        return Ok(out);
    }
    std::fs::create_dir_all(keep_dir)?;
    out.kept_in = Some(keep_dir.to_path_buf());
    for (_, _, p) in &foreign {
        for f in [p.clone(), beside(p, "stream")] {
            if let (true, Some(name)) = (f.is_file(), f.file_name()) {
                move_file(&f, &keep_dir.join(name))?;
            }
        }
        out.removed.push(p.clone());
    }
    let Some(st) = state.as_mut() else { return Ok(out) };
    let mut by_bundle: std::collections::BTreeMap<String, Vec<(u32, usize)>> = std::collections::BTreeMap::new();
    for (i, f) in st.files.iter().enumerate() {
        if let Some((bundle, n)) = f.path.file_name().and_then(|n| patch_name(&n.to_string_lossy())) {
            by_bundle.entry(bundle).or_default().push((n, i));
        }
    }
    for (bundle, mut ours) in by_bundle {
        ours.sort();
        for (slot, (n, i)) in ours.into_iter().enumerate() {
            let slot = slot as u32;
            if slot == n {
                continue;
            }
            let from = here(&contents, &st.files[i]);
            let to = contents.join(format!("{bundle}.patch_{slot}"));
            if !from.is_file() || to.symlink_metadata().is_ok() {
                continue;
            }
            rename_retrying(&from, &to)?;
            if beside(&from, "stream").is_file() {
                rename_retrying(&beside(&from, "stream"), &beside(&to, "stream"))?;
            }
            // The .stream sits in the list as a file of its own, right after its patch.
            let old_stream = beside(&st.files[i].path, "stream");
            st.files[i].path = to.clone();
            if let Some(s) = st.files.iter_mut().find(|f| f.path.file_name() == old_stream.file_name()) {
                s.path = beside(&to, "stream");
            }
            out.renumbered += 1;
        }
    }
    if out.renumbered > 0 {
        write_state(data_dir, st)?;
    }
    Ok(out)
}

pub fn status(game: &GameInstall, data_dir: &Path, known: &[KnownBuild]) -> Result<Status> {
    let deployed = load_state(data_dir)?;
    let foreign = foreign_patches(&game.contents(), deployed.as_ref(), &[])?.into_iter().map(|(_, _, p)| p).collect();
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
        World { _t: t, game, data, opts: DeployOptions { known_builds: known, keep_backups: 10, loader_dir: None, progress: None, loader_only: false } }
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
    fn every_deploy_says_what_it_is_doing_from_start_to_end() {
        let w = world();
        let seen = std::sync::Mutex::new(Vec::new());
        let tell = |p: &DeployProgress| seen.lock().unwrap().push(p.clone());
        let opts = DeployOptions { known_builds: w.opts.known_builds, keep_backups: 10, loader_dir: w.opts.loader_dir.clone(), progress: Some(&tell), loader_only: false };
        deploy(&w.game, &w.data, &opts).unwrap();
        let first_run = std::mem::take(&mut *seen.lock().unwrap());
        assert_eq!((first_run[0].percent, first_run[0].stage), (0, "preparing"));
        assert_eq!((first_run.last().unwrap().percent, first_run.last().unwrap().stage), (100, "done"));
        assert!(first_run.windows(2).all(|p| p[0].percent <= p[1].percent), "{first_run:?}");
        assert!(first_run.iter().any(|p| p.stage == "resources" && p.mod_name.is_some()), "{first_run:?}");
        assert!(first_run.iter().any(|p| p.stage == "scripts" && p.mod_name.is_none()));
        assert!(first_run.iter().any(|p| p.stage == "writing patches"));
        let json = serde_json::to_value(&first_run[0]).unwrap();
        assert_eq!(json, serde_json::json!({ "percent": 0, "mod": null, "stage": "preparing" }));

        // The same deploy again writes nothing, and still reports.
        deploy(&w.game, &w.data, &opts).unwrap();
        let again = seen.lock().unwrap();
        assert_eq!((again[0].stage, again.last().unwrap().stage), ("preparing", "done"));
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
        assert!(settings_text(&w).contains("consent	1
"));
        purge(&w.game, &w.data).unwrap();
        assert!(!patch(&w, "9cf71054ea26dddf.patch_0").exists());
        assert!(settings_text(&w).contains("consent"), "purge leaves settings.txt");
    }

    /// Selects a Sandbox save the way the Saves tab does, by the pointer file the loader settings read.
    fn select_sandbox(w: &World) {
        put(&w.data.join("saves"), "active-sandbox", b"107296691\tsandbox1");
        assert!(crate::saves::active_sandbox(&w.data).is_some());
    }

    fn mods_off(w: &World) {
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-aa", false).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.save(&w.data).unwrap();
    }

    #[test]
    fn a_sandbox_with_no_mods_still_gets_the_loader() {
        let w = world();
        mods_off(&w);
        assert!(deploy(&w.game, &w.data, &w.opts).unwrap().patches.is_empty(), "an Official save with no mods: vanilla");
        assert!(!loader_deployed(&w.game, &w.data));
        select_sandbox(&w);
        let lib = Library::open(&w.data).unwrap();
        assert_eq!(changes_waiting(&lib, &Profile::load(&w.data).unwrap(), &w.data, Some(&w.game.contents())).unwrap(), ChangesWaiting::Waiting);
        assert_eq!(deploy(&w.game, &w.data, &w.opts).unwrap().patches, vec![patch(&w, "9cf71054ea26dddf.patch_0")]);
        assert!(loader_deployed(&w.game, &w.data));
        assert!(settings_text(&w).contains("sandbox\tsandbox1\n"), "the loader reads which Sandbox");
        purge(&w.game, &w.data).unwrap();
        assert!(!patch(&w, "9cf71054ea26dddf.patch_0").exists());
        assert!(!loader_deployed(&w.game, &w.data));
    }

    fn loader_only(w: &World) -> DeployOptions<'static> {
        DeployOptions { known_builds: w.opts.known_builds, keep_backups: 10, loader_dir: None, progress: None, loader_only: true }
    }

    #[test]
    fn a_loader_only_deploy_keeps_the_deployed_mods_and_adds_the_loader() {
        let w = world();
        // Mods deployed with no loader needed: the Official save case.
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(patch(&w, "00000000000000aa.patch_0").exists() && patch(&w, "00000000000000bb.patch_0").exists());
        select_sandbox(&w);
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.save(&w.data).unwrap();
        let r = loader_only(&w);
        deploy(&w.game, &w.data, &r).unwrap();
        assert!(loader_deployed(&w.game, &w.data));
        assert!(patch(&w, "00000000000000aa.patch_0").exists() && patch(&w, "00000000000000bb.patch_0").exists(), "the deployed mods stay, even mod-bb that the profile has off now");
        assert_eq!(load_state(&w.data).unwrap().unwrap().mods, vec!["mod-aa".to_string(), "mod-bb".to_string()]);
        let p = Profile::load(&w.data).unwrap();
        assert!(!p.mods.iter().find(|m| m.id == "mod-bb").unwrap().enabled, "the profile is left as it was");
        let lib = Library::open(&w.data).unwrap();
        assert_eq!(changes_waiting(&lib, &p, &w.data, Some(&w.game.contents())).unwrap(), ChangesWaiting::Waiting, "the Mods tab shows the change waiting");
        // Done once, a second one changes nothing.
        assert!(deploy(&w.game, &w.data, &r).unwrap().unchanged);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(patch(&w, "00000000000000aa.patch_0").exists() && !patch(&w, "00000000000000bb.patch_0").exists() && loader_deployed(&w.game, &w.data));
    }

    #[test]
    fn a_loader_only_deploy_in_a_game_with_no_mods_puts_the_loader_alone() {
        let w = world();
        select_sandbox(&w);
        let r = deploy(&w.game, &w.data, &loader_only(&w)).unwrap();
        assert_eq!(r.patches, vec![patch(&w, "9cf71054ea26dddf.patch_0")]);
        assert!(!patch(&w, "00000000000000aa.patch_0").exists() && !patch(&w, "00000000000000bb.patch_0").exists());
        assert!(loader_deployed(&w.game, &w.data));
        let b = bundle::read_file(&patch(&w, "9cf71054ea26dddf.patch_0")).unwrap();
        assert!(!b.entries.iter().any(|e| e.name_hash == crate::hash::hash_name(&crate::loader::entry_name("mod-aa"))), "no mod script");
        let p = Profile::load(&w.data).unwrap();
        assert!(p.mods.iter().all(|m| m.enabled), "the profile keeps the mods on");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(patch(&w, "00000000000000aa.patch_0").exists() && loader_deployed(&w.game, &w.data));
        purge(&w.game, &w.data).unwrap();
        assert!(!patch(&w, "00000000000000aa.patch_0").exists() && !patch(&w, "9cf71054ea26dddf.patch_0").exists());
    }

    #[test]
    fn blocking_alone_deploys_no_loader() {
        let w = world();
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-aa", false).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.block_public = true;
        p.save(&w.data).unwrap();
        assert!(deploy(&w.game, &w.data, &w.opts).unwrap().patches.is_empty(), "a game without the loader runs nobody's session mods");
    }

    #[test]
    fn a_guardrail_alone_deploys_the_loader() {
        let w = world();
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-aa", false).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.freeze_progress = true;
        p.save(&w.data).unwrap();
        assert_eq!(deploy(&w.game, &w.data, &w.opts).unwrap().patches, vec![patch(&w, "9cf71054ea26dddf.patch_0")]);
        assert!(settings_text(&w).contains("freeze_progress\t1\n"));
        assert!(settings_text(&w).contains("block_achievements\t0\n"));
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
    fn a_deploy_the_game_already_has_writes_nothing_and_a_changed_patch_is_put_back() {
        let w = world();
        let first = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(!first.unchanged);
        let before = snapshot(&w.game.contents());
        let again = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(again.unchanged);
        assert_eq!(again.patches, first.patches);
        assert_eq!(snapshot(&w.game.contents()), before);
        // Another tool changed a patch: it becomes that tool's and stays, and Obsidian's goes after it.
        std::fs::write(&first.patches[0], b"changed").unwrap();
        let fixed = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(!fixed.unchanged);
        assert_eq!(std::fs::read(&first.patches[0]).unwrap(), b"changed");
        assert!(fixed.patches.iter().any(|p| p.to_string_lossy().ends_with("00000000000000aa.patch_1")), "{:?}", fixed.patches);
        assert!(fixed.foreign.contains(&first.patches[0]));
    }

    #[test]
    fn a_deploy_or_purge_while_another_runs_says_so_and_touches_nothing() {
        let w = world();
        let vanilla = snapshot(&w.game.contents());
        let held = hold_lock(&w.data).unwrap();
        assert!(matches!(deploy(&w.game, &w.data, &w.opts), Err(ObsidianError::Busy)));
        assert!(matches!(purge(&w.game, &w.data), Err(ObsidianError::Busy)));
        assert_eq!(snapshot(&w.game.contents()), vanilla);
        drop(held);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        purge(&w.game, &w.data).unwrap();
        assert_eq!(snapshot(&w.game.contents()), vanilla);
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
        bundle::write_file(&patch(&w, "00000000000000aa.patch_3"), &base(8)).unwrap();
        assert!(matches!(deploy(&w.game, &w.data, &w.opts), Err(ObsidianError::ForeignPatchGap { missing: 2, .. })));
        assert!(!patch(&w, "00000000000000bb.patch_0").exists());
    }

    #[test]
    fn a_hand_placed_patch_above_obsidians_keeps_its_number() {
        let w = world();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        // A mod's readme: copy the file in as patch_1, on top of what Obsidian wrote.
        bundle::write_file(&patch(&w, "00000000000000aa.patch_1"), &base(8)).unwrap();
        // Another set of mods, so the deploy writes the patches again.
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("mod-bb", false).unwrap();
        p.save(&w.data).unwrap();
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(!r.unchanged);
        assert!(r.patches.contains(&patch(&w, "00000000000000aa.patch_0")), "{:?}", r.patches);
        assert!(patch(&w, "00000000000000aa.patch_1").exists(), "foreign patches stay");
    }

    #[test]
    fn a_foreign_patch_with_nothing_below_it_is_refused() {
        let w = world();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_2"), &base(8)).unwrap();
        assert!(matches!(deploy(&w.game, &w.data, &w.opts), Err(ObsidianError::ForeignPatchGap { missing: 1, .. })));
    }

    #[test]
    fn removing_other_patches_keeps_a_copy_and_closes_the_gap() {
        let w = world();
        bundle::write_file(&patch(&w, "00000000000000aa.patch_0"), &base(7)).unwrap();
        put(&w.game.contents(), "00000000000000aa.patch_0.stream", b"streamed");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(patch(&w, "00000000000000aa.patch_1").exists(), "Obsidian's patch goes above the other one");
        let listed = foreign_files(&w.game, &w.data).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].name.as_str(), listed[0].bundle.as_str()), ("00000000000000aa.patch_0", "00000000000000aa"));
        let keep = w.data.join("removed-patches");
        let r = remove_foreign(&w.game, &w.data, &keep).unwrap();
        assert_eq!((r.removed.len(), r.renumbered), (1, 1));
        assert!(keep.join("00000000000000aa.patch_0").is_file() && keep.join("00000000000000aa.patch_0.stream").is_file());
        assert!(patch(&w, "00000000000000aa.patch_0").exists() && !patch(&w, "00000000000000aa.patch_1").exists());
        assert!(!patch(&w, "00000000000000aa.patch_0.stream").exists(), "the other patch's stream left with it");
        assert!(foreign_files(&w.game, &w.data).unwrap().is_empty());
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "the moved patch is still the deploy");
        let r = purge(&w.game, &w.data).unwrap();
        assert!(r.kept_changed.is_empty() && !patch(&w, "00000000000000aa.patch_0").exists());
    }

    #[test]
    fn a_deploy_settles_the_game_side_folder_of_an_older_copy() {
        let w = world();
        let old = w.game.saves_dir.with_file_name(crate::settings::LEGACY_DIR);
        put(&old, "logs/a.b.store", b"store");
        let dir = settings::game_side_dir(&w.game.saves_dir);
        assert!(dir.join(settings::CARRY_OVER).is_file(), "copied, and waiting for the deploy");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(!dir.join(settings::CARRY_OVER).exists());
        assert_eq!(std::fs::read(dir.join("logs/a.b.store")).unwrap(), b"store");
        assert!(dir.join(settings::FILE).is_file() && old.join("logs/a.b.store").is_file());
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
    fn an_unknown_build_still_deploys() {
        let w = world();
        bundle::write_file(&w.game.contents().join("00000000000000aa"), &base(5)).unwrap();
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let s = status(&w.game, &w.data, w.opts.known_builds).unwrap();
        assert!(!s.gate.supported());
        assert!(s.deployed.unwrap().build.starts_with("unknown, differs in contents/00000000000000aa"));
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

    /// Adds client mod `opt` (on, last): an entry, a live toggle `fast`, a deploy choice `sky`, a key, and
    /// the resource `mod-aa` changes too.
    fn add_option_mod(w: &World) {
        let d = w.data.join("..").join("stage-opt");
        put(&d, "obsidian.json", br##"{"obsidian":1,"id":"opt","version":"1.0.0","name":{"en":"o"},"entry":"main.lua","scope":"client",
            "options":[{"id":"fast","type":"toggle","default":false},{"id":"sky","type":"choice","default":"day","choices":["day","night"]}],
            "keybinds":[{"id":"toggle","default":"f8"}],
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
    fn changing_a_live_option_is_waiting_until_a_quick_deploy() {
        let w = world();
        add_option_mod(&w);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        change(&w, |p| {
            p.options.entry("opt".into()).or_default().insert("fast".into(), serde_json::json!(true));
        });
        assert_eq!(waiting(&w), ChangesWaiting::Waiting, "an option value");
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(r.unchanged && r.settings_changed, "no patch is rebuilt for an option");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
        change(&w, |p| {
            p.keys.entry("opt".into()).or_default().insert("toggle".into(), "f9".into());
        });
        assert_eq!(waiting(&w), ChangesWaiting::Waiting, "a key");
        change(&w, |p| {
            p.keys.remove("opt");
        });
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "back as deployed");
        let r = deploy(&w.game, &w.data, &w.opts).unwrap();
        assert!(r.unchanged && !r.settings_changed);
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
        change(&w, |p| p.block_public = true);
        assert_eq!(waiting(&w), ChangesWaiting::Vanilla, "blocking alone needs no loader");
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
        let (b, stream) = patch_of([0; 256], planned.clone());
        let order: Vec<_> = b.entries.iter().map(|e| (e.type_hash, e.name_hash)).collect();
        assert_eq!(order, [(4, 9), (5, 1), (5, 2)]);
        assert_eq!(stream, b"ABBBFR");
        let langs: Vec<_> = b.entries[2].variants.iter().map(|v| (v.lang, v.stream_size)).collect();
        assert_eq!(langs, [(0, 3), (0x100, 2)]);
        assert_eq!(b.entries[1].variants, vec![Variant { lang: 0, data: b"plain".to_vec(), stream_size: 0 }]);
        assert!(patch_of([0; 256], planned[1..2].to_vec()).1.is_empty(), "no stream without streamed bytes");
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

    // Sound bank recipes: a game with a bank in two languages, whose French sits past another resource's
    // streamed bytes, and a one-language bank in two bundles.
    const BANK: u64 = 0xf2;
    const SHARED: u64 = 0xf3;

    fn timpani() -> u64 {
        crate::hash::hash_name("timpani_bank")
    }

    fn bank_game(w: &World) {
        use crate::bank::tests::make;
        let (en, en_s) = make(&[(9, b"en-nine", b"EN9"), (2, b"en-two", b""), (5, b"en-five", b"EN5")]);
        let (fr, fr_s) = make(&[(9, b"fr-nine", b"FR9"), (2, b"fr-two", b""), (5, b"fr-five", b"FR5")]);
        let (sh, sh_s) = make(&[(7, b"shared", b"SHS")]);
        let v = |lang, data: &[u8], s: &[u8]| Variant { lang, data: data.to_vec(), stream_size: s.len() as u32 };
        let before = Entry { type_hash: 1, name_hash: 0xe0, flags: 0, variants: vec![Variant { lang: 0, data: b"other".to_vec(), stream_size: 4 }] };
        let bank = Entry { type_hash: timpani(), name_hash: BANK, flags: 3, variants: vec![v(0, &en, &en_s), v(256, &fr, &fr_s)] };
        let shared = |extra: Vec<Entry>| {
            let mut entries = extra;
            entries.push(Entry { type_hash: timpani(), name_hash: SHARED, flags: 0, variants: vec![v(0, &sh, &sh_s)] });
            Bundle { props: [9; 256], entries }
        };
        let c = w.game.contents();
        bundle::write_file(&c.join("00000000000000cc"), &Bundle { props: [9; 256], entries: vec![before, bank] }).unwrap();
        std::fs::write(c.join("00000000000000cc.stream"), [b"GAP!".as_slice(), &en_s, &fr_s].concat()).unwrap();
        bundle::write_file(&c.join("00000000000000dd"), &shared(Vec::new())).unwrap();
        std::fs::write(c.join("00000000000000dd.stream"), &sh_s).unwrap();
        bundle::write_file(&c.join("00000000000000ee"), &shared(Vec::new())).unwrap();
        std::fs::write(c.join("00000000000000ee.stream"), &sh_s).unwrap();
    }

    /// Installs a mod and switches it on, last in the load order. `resources` is the manifest's list.
    fn add(w: &World, id: &str, resources: &str, files: &[(&str, &[u8])]) {
        let d = w._t.path().join(format!("stage-{id}"));
        let json = format!(r##"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},"scope":"client","resources":[{resources}]}}"##);
        put(&d, "obsidian.json", json.as_bytes());
        for (f, b) in files {
            put(&d, f, b);
        }
        let lib = Library::open(&w.data).unwrap();
        lib.install(&d).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.sync(&lib).unwrap();
        p.set_enabled(id, true).unwrap();
        p.save(&w.data).unwrap();
    }

    fn clips_of(id: u64, name: u64, lang: u32, stream: bool) -> String {
        let s = if stream { r#","stream":"c.stream""# } else { "" };
        format!(r##"{{"type":"timpani_bank","name":"#{name:016x}","clips":[{{"clip":"#{id:016x}","lang":{lang},"data":"c.bin"{s}}}]}}"##)
    }

    /// One language of a bank in a deployed patch, with its part of the patch's stream.
    fn deployed(w: &World, bundle_name: &str, name: u64, lang: u32) -> (Vec<u8>, Vec<u8>, u32) {
        let p = bundle::read_file(&patch(w, &format!("{bundle_name}.patch_0"))).unwrap();
        let stream = std::fs::read(patch(w, &format!("{bundle_name}.patch_0.stream"))).unwrap_or_default();
        let mut at = 0;
        for e in &p.entries {
            for v in &e.variants {
                if (e.name_hash, v.lang) == (name, lang) {
                    return (v.data.clone(), stream[at..at + v.stream_size as usize].to_vec(), e.flags);
                }
                at += v.stream_size as usize;
            }
        }
        panic!("no {name:x} in {lang:x}");
    }

    fn audio_of(raw: &[u8], id: u64) -> Vec<u8> {
        let r = crate::bank::rows_of(raw).unwrap().into_iter().find(|r| r.clip == id).unwrap();
        raw[r.off + crate::bank::CLIP_HEADER..r.off + r.size].to_vec()
    }

    #[test]
    fn a_recipe_rebuilds_the_bank_from_the_game_and_leaves_the_rest() {
        use crate::bank::tests::clip;
        let w = world();
        bank_game(&w);
        add(&w, "rock", &clips_of(5, BANK, 256, true), &[("c.bin", &clip(b"new-five")), ("c.stream", b"NEW5")]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let (fr, fr_s, flags) = deployed(&w, "00000000000000cc", BANK, 256);
        assert_eq!(audio_of(&fr, 5), b"new-five");
        assert_eq!(audio_of(&fr, 9), b"fr-nine", "the bank's other clips stay");
        assert_eq!(fr_s, b"FR9NEW5", "the stream has the clips' order");
        assert_eq!(flags, 3, "the game's flags");
        let (en, en_s, _) = deployed(&w, "00000000000000cc", BANK, 0);
        let (en_game, en_game_s) = crate::bank::tests::make(&[(9, b"en-nine", b"EN9"), (2, b"en-two", b""), (5, b"en-five", b"EN5")]);
        assert_eq!((en, en_s), (en_game, en_game_s), "a language the mod doesn't change goes in as the game has it");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "plan and deploy agree");
        let before = deployed(&w, "00000000000000cc", BANK, 256);
        purge(&w.game, &w.data).unwrap();
        assert!(!patch(&w, "00000000000000cc.patch_0").exists());
        assert!(!patch(&w, "00000000000000cc.patch_0.stream").exists());
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(deployed(&w, "00000000000000cc", BANK, 256), before, "it reads the game, not its own last patch");
    }

    #[test]
    fn a_recipe_goes_in_every_bundle_that_holds_the_bank() {
        use crate::bank::tests::clip;
        let w = world();
        bank_game(&w);
        add(&w, "s", &clips_of(7, SHARED, 0, false), &[("c.bin", &clip(b"new"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        for b in ["00000000000000dd", "00000000000000ee"] {
            let (raw, stream, _) = deployed(&w, b, SHARED, 0);
            assert_eq!((audio_of(&raw, 7), stream), (b"new".to_vec(), Vec::new()), "{b}");
        }
    }

    #[test]
    fn mods_on_one_bank_merge_by_clip_and_the_later_wins_a_clip() {
        use crate::bank::tests::clip;
        let w = world();
        bank_game(&w);
        add(&w, "a", &clips_of(5, BANK, 0, false), &[("c.bin", &clip(b"a-five"))]);
        add(&w, "b", &clips_of(9, BANK, 0, false), &[("c.bin", &clip(b"b-nine"))]);
        let conflicts = |w: &World| {
            let lib = Library::open(&w.data).unwrap();
            let c = crate::compose::plan_for(&lib, &Profile::load(&w.data).unwrap(), Some(&w.game.contents())).unwrap();
            c.conflicts.into_iter().filter(|c| c.name_hash == BANK).map(|c| (c.winner, c.losers)).collect::<Vec<_>>()
        };
        assert!(conflicts(&w).is_empty(), "different clips combine");
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let (en, _, _) = deployed(&w, "00000000000000cc", BANK, 0);
        assert_eq!((audio_of(&en, 5), audio_of(&en, 9)), (b"a-five".to_vec(), b"b-nine".to_vec()));
        add(&w, "c", &clips_of(5, BANK, 0, false), &[("c.bin", &clip(b"c-five"))]);
        assert_eq!(conflicts(&w), [("c".to_string(), vec!["a".to_string()])]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(audio_of(&deployed(&w, "00000000000000cc", BANK, 0).0, 5), b"c-five");
        add(&w, "d", &clips_of(5, BANK, 0, false), &[("c.bin", &clip(b"c-five"))]);
        assert_eq!(conflicts(&w), [("c".to_string(), vec!["a".to_string()])], "the same bytes lose nothing");
    }

    #[test]
    fn a_recipe_goes_on_a_whole_bank_below_it_and_loses_to_one_above() {
        use crate::bank::tests::{clip, make};
        let w = world();
        bank_game(&w);
        let (whole, whole_s) = make(&[(5, b"whole-five", b"W5"), (9, b"whole-nine", b"")]);
        let res = format!(r##"{{"type":"timpani_bank","name":"#{BANK:016x}","bundle":"00000000000000cc","data":"w.bin","stream":"w.stream"}}"##);
        add(&w, "whole", &res, &[("w.bin", &whole), ("w.stream", &whole_s)]);
        add(&w, "top", &clips_of(9, BANK, 0, false), &[("c.bin", &clip(b"top-nine"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let (en, _, _) = deployed(&w, "00000000000000cc", BANK, 0);
        assert_eq!((audio_of(&en, 5), audio_of(&en, 9)), (b"whole-five".to_vec(), b"top-nine".to_vec()));
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
        add(&w, "miss", &clips_of(2, BANK, 0, false), &[("c.bin", &clip(b"x"))]);
        let e = deploy(&w.game, &w.data, &w.opts).unwrap_err();
        assert!(matches!(e, ObsidianError::RecipeClip { ref mod_id, .. } if mod_id == "miss"), "a clip the whole bank lacks: {e}");
        let lib = Library::open(&w.data).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.set_enabled("miss", false).unwrap();
        p.save(&w.data).unwrap();
        add(&w, "whole2", &res.replace("w.bin", "w2.bin").replace("w.stream", "w2.stream"), &[("w2.bin", &make(&[(5, b"second", b"")]).0), ("w2.stream", b"")]);
        let c = crate::compose::plan_for(&lib, &Profile::load(&w.data).unwrap(), Some(&w.game.contents())).unwrap();
        let c = c.conflicts.iter().find(|c| c.name_hash == BANK).unwrap();
        assert_eq!((c.winner.as_str(), c.losers.clone()), ("whole2", vec!["whole".to_string(), "top".to_string()]));
    }

    #[test]
    fn a_recipe_the_game_cant_take_is_refused_and_the_last_deploy_stays() {
        use crate::bank::tests::clip;
        let refused = |res: String, file: Vec<u8>| {
            let w = world();
            bank_game(&w);
            deploy(&w.game, &w.data, &w.opts).unwrap();
            let before = snapshot(&w.game.contents());
            add(&w, "m", &res, &[("c.bin", &file)]);
            let e = deploy(&w.game, &w.data, &w.opts).unwrap_err();
            assert_eq!(snapshot(&w.game.contents()), before, "{e}");
            assert_eq!(waiting(&w), ChangesWaiting::Waiting, "{e}");
            e
        };
        assert!(matches!(refused(clips_of(77, BANK, 0, false), clip(b"x")), ObsidianError::RecipeClip { ref clip, lang: 0, .. } if clip == "#000000000000004d"));
        assert!(matches!(refused(clips_of(5, BANK, 512, false), clip(b"x")), ObsidianError::RecipeLanguage { lang: 512, .. }));
        assert!(matches!(refused(clips_of(5, 0xbad, 0, false), clip(b"x")), ObsidianError::RecipeBank { ref bank, .. } if bank == "#0000000000000bad"));
        assert!(matches!(refused(clips_of(5, BANK, 0, false), b"not a clip".to_vec()), ObsidianError::RecipeFile { ref file, .. } if file == "c.bin"));
    }

    #[test]
    fn a_changed_bank_in_the_game_counts_as_changes_waiting() {
        use crate::bank::tests::{clip, make};
        let w = world();
        bank_game(&w);
        add(&w, "s", &clips_of(7, SHARED, 0, false), &[("c.bin", &clip(b"new"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
        let (sh, _) = make(&[(7, b"patched by steam", b"")]);
        let v = Variant { lang: 0, data: sh, stream_size: 0 };
        let path = w.game.contents().join("00000000000000dd");
        bundle::write_file(&path, &Bundle { props: [9; 256], entries: vec![Entry { type_hash: timpani(), name_hash: SHARED, flags: 0, variants: vec![v] }] }).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "a new deploy is all it takes");
    }

    // The overlay (overlay.rs): the same game with a master in a bundle of its own, naming clips 9, 2, 5
    // and 7 of the banks, and one no bank has.
    const BOOT: &str = "00000000000000ff";

    fn master_game(w: &World) -> Vec<u8> {
        bank_game(w);
        let m = crate::overlay::tests::master(&[Some(9), Some(2), None, Some(5), Some(7), Some(0x77), Some(5)]);
        let entry = Entry { type_hash: crate::hash::hash_name("timpani_master"), name_hash: 0x3a, flags: 0, variants: vec![Variant { lang: 0, data: m.clone(), stream_size: 0 }] };
        let other = Entry { type_hash: 1, name_hash: 0x3b, flags: 0, variants: vec![Variant { lang: 0, data: b"boot".to_vec(), stream_size: 0 }] };
        bundle::write_file(&w.game.contents().join(BOOT), &Bundle { props: [9; 256], entries: vec![other, entry] }).unwrap();
        m
    }

    fn new_id(clip: u64) -> u64 {
        crate::hash::hash_name(&format!("obsidian/clip/{clip:016x}"))
    }

    /// The name of Obsidian's bank of new clips beside a bank of the game.
    fn clips_beside(bank: u64) -> u64 {
        crate::hash::hash_name(&format!("{}/{bank:016x}", crate::overlay::BANK_NAME))
    }

    /// Every resource of a bundle file, (type, name, language, bytes, bytes in its `.stream`).
    fn variants_of(path: &Path) -> Vec<(u64, u64, u32, Vec<u8>, Vec<u8>)> {
        let b = bundle::read_file(path).unwrap();
        let stream = std::fs::read(beside_file(path, "stream")).unwrap_or_default();
        let mut at = 0;
        let mut out = Vec::new();
        for e in &b.entries {
            for v in &e.variants {
                out.push((e.type_hash, e.name_hash, v.lang, v.data.clone(), stream[at..at + v.stream_size as usize].to_vec()));
                at += v.stream_size as usize;
            }
        }
        out
    }

    fn beside_file(path: &Path, suffix: &str) -> PathBuf {
        super::beside(path, suffix)
    }

    /// The clips the deployed master names, in record order.
    fn deployed_master(w: &World) -> Vec<u64> {
        let p = bundle::read_file(&patch(w, &format!("{BOOT}.patch_0"))).unwrap();
        let e = p.entries.iter().find(|e| e.type_hash == crate::hash::hash_name("timpani_master")).expect("the master");
        crate::overlay::clip_places(&e.variants[0].data).unwrap().into_iter().map(|p| p.1).collect()
    }

    /// The languages of Obsidian's bank beside `bank`, in the patch of `bundle_name`.
    fn overlay_langs(w: &World, bundle_name: &str, bank: u64) -> Vec<u32> {
        let p = bundle::read_file(&patch(w, &format!("{bundle_name}.patch_0"))).unwrap();
        p.entries.iter().find(|e| e.name_hash == clips_beside(bank)).expect("the bank of new clips").variants.iter().map(|v| v.lang).collect()
    }

    fn overlay_clip(w: &World, bundle_name: &str, bank: u64, lang: u32, clip: u64) -> (Vec<u8>, Vec<u8>) {
        let (raw, stream, flags) = deployed(w, bundle_name, clips_beside(bank), lang);
        assert_eq!(flags, 0);
        let r = crate::bank::rows_of(&raw).unwrap().into_iter().find(|r| r.clip == new_id(clip)).expect("the clip");
        (raw[r.off + crate::bank::CLIP_HEADER..r.off + r.size].to_vec(), stream[r.stream_off..r.stream_off + r.stream_size].to_vec())
    }

    /// What the game plays in `lang` for the master's clip record `record`, picked as the game picks: each
    /// bundle's resources with its patch over them, in the language's own variant or else language 0's,
    /// except in the boot bundle, which loads before the language is set and always takes language 0 (round
    /// J). Then the first bank that has the clip the master names. Returns the clip's audio and its stream.
    fn heard(w: &World, lang: u32, record: usize) -> (Vec<u8>, Vec<u8>) {
        let contents = w.game.contents();
        let mut files: Vec<String> = std::fs::read_dir(&contents).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.len() == 16).collect();
        files.sort();
        let mut master = None;
        let mut banks = Vec::new();
        for name in files {
            let pick = |list: Vec<(u64, u64, u32, Vec<u8>, Vec<u8>)>| {
                let mut by: BTreeMap<(u64, u64), Vec<(u32, Vec<u8>, Vec<u8>)>> = BTreeMap::new();
                for (t, n, l, d, s) in list {
                    by.entry((t, n)).or_default().push((l, d, s));
                }
                by
            };
            let mut here = pick(variants_of(&contents.join(&name)));
            let patch = contents.join(format!("{name}.patch_0"));
            if patch.is_file() {
                here.extend(pick(variants_of(&patch)));
            }
            let want = if name == BOOT { 0 } else { lang };
            for ((t, _), vs) in here {
                let (_, d, s) = vs.iter().find(|v| v.0 == want).or_else(|| vs.iter().find(|v| v.0 == 0)).unwrap().clone();
                if t == crate::hash::hash_name("timpani_master") {
                    master = Some(d);
                } else if t == timpani() {
                    banks.push((d, s));
                }
            }
        }
        let clip = crate::overlay::clip_places(&master.expect("a master")).unwrap()[record].1;
        for (raw, stream) in banks {
            if let Some(r) = crate::bank::rows_of(&raw).unwrap().into_iter().find(|r| r.clip == clip) {
                return (raw[r.off + crate::bank::CLIP_HEADER..r.off + r.size].to_vec(), stream[r.stream_off..r.stream_off + r.stream_size].to_vec());
            }
        }
        panic!("no loaded bank has clip {clip:#x}: silence")
    }

    #[test]
    fn a_recipe_deploys_as_new_clips_beside_the_bank_and_the_master_names_them() {
        use crate::bank::tests::clip;
        let w = world();
        master_game(&w);
        let before = snapshot(&w.game.contents());
        add(&w, "rock", &clips_of(5, BANK, 256, true), &[("c.bin", &clip(b"new-five")), ("c.stream", b"NEW5")]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(deployed_master(&w), [9, 2, new_id(5), 7, 0x77, new_id(5)], "every record of the clip names the new one");
        let boot = bundle::read_file(&patch(&w, &format!("{BOOT}.patch_0"))).unwrap();
        assert_eq!(boot.entries.len(), 1, "the boot bundle gets the master alone: a bank there would load before the language is set");
        let cc = bundle::read_file(&patch(&w, "00000000000000cc.patch_0")).unwrap();
        assert_eq!(cc.entries.iter().map(|e| e.name_hash).collect::<Vec<_>>(), [clips_beside(BANK)], "the game's bank isn't patched");
        assert_eq!(overlay_langs(&w, "00000000000000cc", BANK), [0, 256], "French differs from the rest");
        assert_eq!(overlay_clip(&w, "00000000000000cc", BANK, 256, 5), (b"new-five".to_vec(), b"NEW5".to_vec()));
        assert_eq!(overlay_clip(&w, "00000000000000cc", BANK, 0, 5), (b"en-five".to_vec(), b"EN5".to_vec()));
        assert_eq!(std::fs::read(patch(&w, "00000000000000cc.patch_0.stream")).unwrap(), b"EN5NEW5");
        assert_eq!(heard(&w, 256, 2), (b"new-five".to_vec(), b"NEW5".to_vec()), "French hears the mod");
        assert_eq!(heard(&w, 0, 2), (b"en-five".to_vec(), b"EN5".to_vec()), "English hears the game");
        assert_eq!(heard(&w, 0, 0), (b"en-nine".to_vec(), b"EN9".to_vec()), "a clip no mod changes is the game's");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate, "plan and deploy agree");
        purge(&w.game, &w.data).unwrap();
        assert_eq!(snapshot(&w.game.contents()), before);
    }

    #[test]
    fn a_clip_changed_in_english_alone_plays_the_games_own_in_french() {
        use crate::bank::tests::clip;
        let w = world();
        master_game(&w);
        add(&w, "beep", &clips_of(5, BANK, 0, true), &[("c.bin", &clip(b"beep")), ("c.stream", b"BEEP")]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(overlay_langs(&w, "00000000000000cc", BANK), [0, 256]);
        assert_eq!(heard(&w, 0, 2), (b"beep".to_vec(), b"BEEP".to_vec()));
        assert_eq!(heard(&w, 256, 2), (b"fr-five".to_vec(), b"FR5".to_vec()), "French keeps the game's own clip");
        assert_eq!(heard(&w, 0x4000, 2), (b"beep".to_vec(), b"BEEP".to_vec()), "a language the bank lacks takes language 0, as the game does");
    }

    #[test]
    fn a_clip_the_same_in_every_language_takes_one_variant() {
        use crate::bank::tests::clip;
        let w = world();
        master_game(&w);
        let both = format!(r##"{{"type":"timpani_bank","name":"#{BANK:016x}","clips":[{{"clip":"#0000000000000005","data":"c.bin"}},{{"clip":"#0000000000000005","lang":256,"data":"c.bin"}}]}}"##);
        add(&w, "beep", &both, &[("c.bin", &clip(b"beep"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(overlay_langs(&w, "00000000000000cc", BANK), [0], "French falls back to the first variant");
        assert_eq!(heard(&w, 256, 2), (b"beep".to_vec(), Vec::new()));
    }

    #[test]
    fn a_bank_in_two_bundles_gets_new_clips_beside_it_in_both() {
        use crate::bank::tests::clip;
        let w = world();
        master_game(&w);
        add(&w, "s", &clips_of(7, SHARED, 0, false), &[("c.bin", &clip(b"new"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        for b in ["00000000000000dd", "00000000000000ee"] {
            let p = bundle::read_file(&patch(&w, &format!("{b}.patch_0"))).unwrap();
            assert_eq!(p.entries.iter().map(|e| e.name_hash).collect::<Vec<_>>(), [clips_beside(SHARED)], "{b}");
            assert_eq!((overlay_langs(&w, b, SHARED), overlay_clip(&w, b, SHARED, 0, 7)), (vec![0], (b"new".to_vec(), Vec::new())));
        }
        assert_eq!(deployed_master(&w), [9, 2, 5, new_id(7), 0x77, 5]);
        assert!(!patch(&w, "00000000000000cc.patch_0").exists(), "a bank no mod changes keeps its bundle as it is");
    }

    #[test]
    fn mods_on_one_clip_the_later_wins_in_the_overlay_too() {
        use crate::bank::tests::clip;
        let w = world();
        master_game(&w);
        add(&w, "a", &clips_of(5, BANK, 0, false), &[("c.bin", &clip(b"a-five"))]);
        add(&w, "b", &clips_of(9, BANK, 0, false), &[("c.bin", &clip(b"b-nine"))]);
        add(&w, "c", &clips_of(5, BANK, 0, false), &[("c.bin", &clip(b"c-five"))]);
        let lib = Library::open(&w.data).unwrap();
        let c = crate::compose::plan_for(&lib, &Profile::load(&w.data).unwrap(), Some(&w.game.contents())).unwrap();
        let c: Vec<_> = c.conflicts.into_iter().filter(|c| c.name_hash == BANK).map(|c| (c.winner, c.losers)).collect();
        assert_eq!(c, [("c".to_string(), vec!["a".to_string()])]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!((heard(&w, 0, 2).0, heard(&w, 0, 0).0), (b"c-five".to_vec(), b"b-nine".to_vec()));
        assert_eq!(heard(&w, 256, 0).0, b"fr-nine", "French keeps the game's lines");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
    }

    #[test]
    fn a_whole_bank_below_still_deploys_and_gives_the_clips_it_has() {
        use crate::bank::tests::{clip, make};
        let w = world();
        master_game(&w);
        let (whole, whole_s) = make(&[(5, b"whole-five", b"W5"), (9, b"whole-nine", b"")]);
        let res = format!(r##"{{"type":"timpani_bank","name":"#{BANK:016x}","bundle":"00000000000000cc","data":"w.bin","stream":"w.stream"}}"##);
        add(&w, "whole", &res, &[("w.bin", &whole), ("w.stream", &whole_s)]);
        add(&w, "top", &clips_of(9, BANK, 0, false), &[("c.bin", &clip(b"top-nine"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        let (en, en_s, _) = deployed(&w, "00000000000000cc", BANK, 0);
        assert_eq!((en, en_s), (whole, whole_s), "the whole bank goes in as the mod has it");
        assert_eq!((heard(&w, 0, 0).0, heard(&w, 256, 0).0), (b"top-nine".to_vec(), b"top-nine".to_vec()), "the mod's bank has one language");
        assert_eq!(heard(&w, 256, 2).0, b"whole-five", "the clips the top mod leaves are the whole bank's");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
    }

    #[test]
    fn a_whole_master_of_a_mod_takes_the_new_ids() {
        use crate::bank::tests::clip;
        let w = world();
        master_game(&w);
        let theirs = crate::overlay::tests::master(&[Some(5), Some(2), Some(2)]);
        let res = format!(r##"{{"type":"timpani_master","name":"#000000000000003a","bundle":"{BOOT}","data":"m.bin"}}"##);
        add(&w, "sounds", &res, &[("m.bin", &theirs)]);
        add(&w, "rock", &clips_of(5, BANK, 0, false), &[("c.bin", &clip(b"new-five"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(deployed_master(&w), [new_id(5), 2, 2]);
        assert_eq!(heard(&w, 0, 0).0, b"new-five");
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
    }

    #[test]
    fn an_overlay_the_game_cant_take_is_refused_and_the_last_deploy_stays() {
        use crate::bank::tests::clip;
        let refused = |res: String, file: Vec<u8>| {
            let w = world();
            master_game(&w);
            deploy(&w.game, &w.data, &w.opts).unwrap();
            let before = snapshot(&w.game.contents());
            add(&w, "m", &res, &[("c.bin", &file)]);
            let e = deploy(&w.game, &w.data, &w.opts).unwrap_err();
            assert_eq!(snapshot(&w.game.contents()), before, "{e}");
            assert_eq!(waiting(&w), ChangesWaiting::Waiting, "{e}");
            e
        };
        assert!(matches!(refused(clips_of(77, BANK, 0, false), clip(b"x")), ObsidianError::RecipeClip { ref clip, lang: 0, .. } if clip == "#000000000000004d"));
        assert!(matches!(refused(clips_of(5, BANK, 512, false), clip(b"x")), ObsidianError::RecipeLanguage { lang: 512, .. }));
        assert!(matches!(refused(clips_of(5, 0xbad, 0, false), clip(b"x")), ObsidianError::RecipeBank { ref bank, .. } if bank == "#0000000000000bad"));
        assert!(matches!(refused(clips_of(5, BANK, 0, false), b"not a clip".to_vec()), ObsidianError::RecipeFile { ref file, .. } if file == "c.bin"));
    }

    #[test]
    fn a_game_update_to_a_bank_or_the_master_is_changes_waiting() {
        use crate::bank::tests::{clip, make};
        let w = world();
        let m = master_game(&w);
        add(&w, "s", &clips_of(7, SHARED, 0, false), &[("c.bin", &clip(b"new"))]);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
        let (sh, _) = make(&[(7, b"patched by steam", b"")]);
        let path = w.game.contents().join("00000000000000dd");
        bundle::write_file(&path, &Bundle { props: [9; 256], entries: vec![Entry { type_hash: timpani(), name_hash: SHARED, flags: 0, variants: vec![Variant { lang: 0, data: sh, stream_size: 0 }] }] }).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Waiting);
        deploy(&w.game, &w.data, &w.opts).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::UpToDate);
        let entry = Entry { type_hash: crate::hash::hash_name("timpani_master"), name_hash: 0x3a, flags: 0, variants: vec![Variant { lang: 0, data: [m.as_slice(), b"\0\0\0\0"].concat(), stream_size: 0 }] };
        bundle::write_file(&w.game.contents().join(BOOT), &Bundle { props: [9; 256], entries: vec![entry] }).unwrap();
        assert_eq!(waiting(&w), ChangesWaiting::Waiting, "a new master");
    }
}
