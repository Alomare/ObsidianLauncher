//! The Saves library. Per Steam account under `<data>/saves/<account>/`:
//!
//! ```text
//! library.json       the list of saves and the active Official id
//! <id>/common.sav    the Official slot of that save, local format
//! <id>/user.sav
//! <id>/mod.obsidian  per-save mod data of the slot, when any
//! ```
//!
//! The active Official lives in the game's own files; other Official saves only in the library. A
//! Sandbox lives in the game's saves folder (`helldivers_sandbox_<id>.sav`), where the loader points
//! the game, and in the library only as a name. `<data>/saves/active-sandbox` holds
//! `<account>\t<id>` while a Sandbox is selected.

pub mod account;
pub mod catalog;
pub mod codec;
pub mod editor;
pub mod mod_data;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::game::GameInstall;
use crate::library::{Library, Profile};
use crate::paths::write_atomic;
use crate::{ObsidianError, Result};
use codec::{Format, SaveFile, Table, Value, decode_file, encode_file, read_file};

const BLANK_COMMON: &[u8] = include_bytes!("../../assets/blank-common.sav");
const BLANK_USER: &[u8] = include_bytes!("../../assets/blank-user.sav");

const COMMON_QUEUES: [&str; 3] = ["mission_stats", "mission_stats_to_upload", "completed_planets"];
const USER_QUEUES: [&str; 4] = ["leaderboard_stats", "leaderboard_stats_to_upload", "leaderboard_provinggrounds_stats", "leaderboard_provinggrounds_stats_to_upload"];
const COUNTERS: [&str; 4] = ["planet", "mission_stats", "leaderboard_stats", "leaderboard_provinggrounds_stats"];
/// The flags of every tutorial the game runs once, all in the common file: boot camp, the galaxy
/// map tutorials, the admiral introduction and hints on the ship, and the hellpod arrows. Without
/// `dia_admiral_introduction_performed` the ship opens the galaxy map on first boarding.
const TUTORIALS: [&str; 12] = [
    "tutorial_finished",
    "galcamp_tutorial_completed_attack_event",
    "galcamp_tutorial_completed_defend_event",
    "galcamp_tutorial_completed_defend_superearth_event",
    "galcamp_tutorial_completed_mission",
    "galcamp_tutorial_completed_planet",
    "galcamp_tutorial_completed_recap",
    "galcamp_tutorial_completed_region",
    "dia_admiral_introduction_performed",
    "dia_admiral_armory_hint_performed",
    "dia_admiral_multiplayer_hint_performed",
    "has_shown_pod_arrows",
];

/// As many backups as a deploy keeps.
const KEEP_BACKUPS: usize = 10;
const INDEX: &str = "library.json";
const ACTIVE_SANDBOX: &str = "active-sandbox";
const OFFICIAL_MOD_DATA: &str = "helldivers_official.obsidian";
const SAVE_EXPORT_FILE: &str = "obsidian-save.json";
/// The most an imported save part may be. A real common file is under 20 KB.
const MAX_IMPORT: u64 = 1 << 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaveKind {
    Official,
    Sandbox,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveEntry {
    pub id: String,
    pub name: String,
    pub kind: SaveKind,
    /// RFC 3339.
    pub created: String,
    /// The Official save this was snapshotted from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_of: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Index {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_official: Option<String>,
    #[serde(default)]
    saves: Vec<SaveEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStats {
    pub rank: u32,
    pub xp: f64,
    pub samples: f64,
    pub unlocks: usize,
    pub kills: f64,
    pub deaths: f64,
    pub missions_fought: f64,
    pub missions_won: f64,
    pub time_played: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRow {
    pub id: String,
    pub name: String,
    pub kind: SaveKind,
    pub created: String,
    /// When the game last wrote this save. None for an Official save resting in the library.
    pub last_played: Option<String>,
    pub snapshot_of: Option<String>,
    /// The save the game boots next.
    pub active: bool,
    /// An Official save whose data is in the game's own files right now.
    pub live: bool,
    pub stats: Option<SaveStats>,
    /// Why the save can't be read, when it can't.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveList {
    pub account: String,
    pub active: Option<String>,
    pub saves: Vec<SaveRow>,
}

/// A save: the common file and the user file.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub common: SaveFile,
    pub user: SaveFile,
}

pub fn blank_pair() -> Pair {
    Pair { common: decode_file(BLANK_COMMON).expect("built-in save"), user: decode_file(BLANK_USER).expect("built-in save") }
}

/// Marks every tutorial done, as a new Sandbox starts.
pub fn complete_tutorials(p: &mut Pair) {
    for k in TUTORIALS {
        p.common.root.set(k, Value::Bool(1));
    }
}

/// Removes the upload queues, so nothing queued goes to the server after a restore or an import.
pub fn strip_queues(p: &mut Pair) {
    for k in COMMON_QUEUES {
        p.common.root.remove(k);
    }
    for k in USER_QUEUES {
        p.user.root.remove(k);
    }
}

/// Whether `key` is a server counter, as `backend_planet_index_win32`.
fn is_counter(key: &str) -> bool {
    let Some(rest) = key.strip_prefix("backend_") else { return false };
    COUNTERS.iter().any(|c| rest.strip_prefix(*c).is_some_and(|r| r.starts_with("_index_")))
}

/// Takes the higher of each server counter from `from` into `to`, so the server does not refuse the
/// next upload from a restored save.
pub fn carry_counters(to: &mut Table, from: &Table) {
    for (k, v) in &from.0 {
        let (Some(name), Some(n)) = (k.as_str(), v.as_number()) else { continue };
        if is_counter(&name) && to.number(&name).is_none_or(|t| t < n) {
            to.set(&name, Value::Number(n));
        }
    }
}

/// Removes paid DLC the account does not own from `item_unlocks` of a user table.
pub fn remove_unowned_dlc(user: &mut Table, owned: &BTreeSet<String>) {
    let cat = catalog::get();
    let Some(t) = user.get("item_unlocks").and_then(Value::as_table).cloned() else { return };
    let keep: Vec<String> = t.strings().into_iter().filter(|n| !cat.is_locked(n, owned)).collect();
    if keep.len() != t.strings().len() {
        user.table_mut("item_unlocks").set_strings(&keep);
    }
}

fn stats_of(p: &Pair) -> SaveStats {
    let u = &p.user.root;
    let life = u.table("lifetime_stats");
    let stat = |k: &str| life.and_then(|t| t.number(k)).unwrap_or(0.0);
    let xp = u.number("player_xp").unwrap_or(0.0);
    SaveStats {
        rank: catalog::get().rank_of(xp),
        xp,
        samples: u.number("research_samples").unwrap_or(0.0),
        unlocks: u.table("item_unlocks").map_or(0, |t| t.strings().len()),
        kills: stat("kills"),
        deaths: stat("deaths"),
        missions_fought: stat("missions_fought"),
        missions_won: stat("missions_won"),
        time_played: stat("time_played"),
    }
}

/// The selected Sandbox, whichever account: what settings read.
pub fn active_sandbox(data: &Path) -> Option<String> {
    read_pointer(data).map(|(_, id)| id)
}

fn pointer_path(data: &Path) -> PathBuf {
    data.join("saves").join(ACTIVE_SANDBOX)
}

fn read_pointer(data: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(pointer_path(data)).ok()?;
    let (account, id) = text.trim().split_once('\t')?;
    Some((account.to_string(), id.to_string()))
}

fn check_name(name: &str) -> Result<()> {
    let bad = name.trim().is_empty() || name.chars().count() > 40 || name.chars().any(char::is_control);
    if bad { Err(ObsidianError::SaveName(name.to_string())) } else { Ok(()) }
}

fn write_verified(path: &Path, file: &SaveFile, format: Format) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let bytes = encode_file(file, format);
    write_atomic(path, &bytes)?;
    if std::fs::read(path)? != bytes {
        return Err(ObsidianError::BadSave(format!("{} did not read back as written", path.display())));
    }
    decode_file(&bytes)?;
    Ok(())
}

fn write_slot_files(common: &Path, user: &Path, p: &Pair, format: Format) -> Result<()> {
    write_verified(common, &p.common, format)?;
    write_verified(user, &p.user, format)
}

fn copy_or_remove(from: &Path, to: &Path) -> Result<()> {
    if from.is_file() {
        let bytes = std::fs::read(from)?;
        if let Some(p) = to.parent() {
            std::fs::create_dir_all(p)?;
        }
        write_atomic(to, &bytes)?;
    } else {
        match std::fs::remove_file(to) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// The newest of `paths` that reads, or None.
fn newest_valid(paths: &[PathBuf]) -> Option<SaveFile> {
    let mut best: Option<(std::time::SystemTime, SaveFile)> = None;
    for p in paths {
        let Ok(when) = std::fs::metadata(p).and_then(|m| m.modified()) else { continue };
        let Ok(file) = read_file(p) else { continue };
        if best.as_ref().is_none_or(|(b, _)| when > *b) {
            best = Some((when, file));
        }
    }
    best.map(|(_, f)| f)
}

/// One Steam account's saves.
pub struct Saves {
    pub(crate) data: PathBuf,
    pub(crate) dir: PathBuf,
    account: String,
    uid: String,
    game: GameInstall,
    remote: Option<PathBuf>,
    running: fn() -> bool,
}

impl Saves {
    /// The active Steam account's saves. A selected Sandbox of another account is cleared.
    pub fn open(data: &Path, game: &GameInstall, running: fn() -> bool) -> Result<Saves> {
        let account = account::detect(&crate::game::steam_roots(), &game.cloud_saves, &game.saves_dir).ok_or(ObsidianError::NoSteamAccount)?;
        let s = Saves::for_account(data, game, &account, running);
        if read_pointer(data).is_some_and(|(a, _)| a != account) && !running() {
            s.set_pointer(None)?;
            s.write_settings()?;
        }
        Ok(s)
    }

    pub fn for_account(data: &Path, game: &GameInstall, account: &str, running: fn() -> bool) -> Saves {
        let remote = game.cloud_saves.iter().find(|r| r.ancestors().nth(2).and_then(Path::file_name).is_some_and(|n| n.to_string_lossy() == account)).cloned();
        Saves { data: data.to_path_buf(), dir: data.join("saves").join(account), account: account.to_string(), uid: account::uid(account), game: game.clone(), remote, running }
    }

    pub fn account(&self) -> &str {
        &self.account
    }

    pub(crate) fn closed(&self) -> Result<()> {
        if (self.running)() { Err(ObsidianError::GameRunning) } else { Ok(()) }
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join(INDEX)
    }

    fn read_index(&self) -> Result<Index> {
        match std::fs::read(self.index_path()) {
            Ok(b) => Ok(serde_json::from_slice(&b)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Index::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn save_index(&self, idx: &Index) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        write_atomic(&self.index_path(), (serde_json::to_string_pretty(idx)? + "\n").as_bytes())?;
        Ok(())
    }

    fn set_pointer(&self, id: Option<&str>) -> Result<()> {
        let path = pointer_path(&self.data);
        match id {
            Some(id) => {
                if let Some(p) = path.parent() {
                    std::fs::create_dir_all(p)?;
                }
                write_atomic(&path, format!("{}\t{id}", self.account).as_bytes())?;
            }
            None => match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            },
        }
        Ok(())
    }

    fn write_settings(&self) -> Result<()> {
        let lib = Library::open(&self.data)?;
        let mut profile = Profile::load(&self.data)?;
        profile.sync(&lib)?;
        crate::edit::write_settings(&lib, &profile, &self.game.saves_dir)
    }

    fn slot_common(&self, id: &str) -> PathBuf {
        self.dir.join(id).join("common.sav")
    }
    fn slot_user(&self, id: &str) -> PathBuf {
        self.dir.join(id).join("user.sav")
    }
    fn slot_mod(&self, id: &str) -> PathBuf {
        self.dir.join(id).join("mod.obsidian")
    }
    fn official_mod(&self) -> PathBuf {
        self.game.saves_dir.join(OFFICIAL_MOD_DATA)
    }

    fn sandbox_common(&self, id: &str) -> PathBuf {
        self.game.saves_dir.join(format!("helldivers_sandbox_{id}.sav"))
    }
    fn sandbox_user(&self, id: &str) -> PathBuf {
        self.game.saves_dir.join(format!("helldivers_sandbox_{id}_user.sav"))
    }
    fn sandbox_old_common(&self, id: &str) -> PathBuf {
        self.game.saves_dir.join(format!("helldivers_sandbox_{id}.old"))
    }
    fn sandbox_old_user(&self, id: &str) -> PathBuf {
        self.game.saves_dir.join(format!("helldivers_sandbox_{id}_user.old"))
    }
    pub(crate) fn sandbox_mod(&self, id: &str) -> PathBuf {
        self.game.saves_dir.join(format!("helldivers_sandbox_{id}.obsidian"))
    }

    fn local(&self, suffix: &str) -> PathBuf {
        self.game.saves_dir.join(format!("helldivers_steam_{}{suffix}", self.uid))
    }

    fn cloud_common_name(&self) -> String {
        "helldivers_save".to_string()
    }
    fn cloud_user_name(&self) -> String {
        format!("helldivers_save{}", self.uid)
    }

    /// The game's current files: per file the newest of the local copy and the Cloud copy that read.
    fn capture_live(&self) -> Option<Pair> {
        let mut common = vec![self.local(".sav")];
        let mut user = vec![self.local("_user.sav")];
        if let Some(r) = &self.remote {
            common.insert(0, r.join(self.cloud_common_name()));
            user.insert(0, r.join(self.cloud_user_name()));
        }
        let (c, u) = (newest_valid(&common), newest_valid(&user));
        if c.is_none() && u.is_none() {
            return None;
        }
        let blank = blank_pair();
        Some(Pair { common: c.unwrap_or(blank.common), user: u.unwrap_or(blank.user) })
    }

    /// The library with the current game files adopted as an Official save the first time it is used.
    fn ensure(&self) -> Result<Index> {
        let mut idx = self.read_index()?;
        if idx.saves.is_empty()
            && let Some(pair) = self.capture_live()
        {
            let id = self.new_id(&idx);
            self.write_slot(&id, &pair)?;
            copy_or_remove(&self.official_mod(), &self.slot_mod(&id))?;
            idx.saves.push(SaveEntry { id: id.clone(), name: "Main".to_string(), kind: SaveKind::Official, created: now(), snapshot_of: None });
            idx.active_official = Some(id);
            self.save_index(&idx)?;
        }
        Ok(idx)
    }

    fn new_id(&self, idx: &Index) -> String {
        use sha2::{Digest, Sha256};
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        (0u32..)
            .map(|i| Sha256::digest(format!("{nanos}:{i}:{}", self.account)).iter().take(4).map(|b| format!("{b:02x}")).collect::<String>())
            .find(|id| !idx.saves.iter().any(|s| s.id == *id) && !self.sandbox_common(id).exists() && !self.dir.join(id).exists())
            .expect("a free id")
    }

    fn write_slot(&self, id: &str, p: &Pair) -> Result<()> {
        write_slot_files(&self.slot_common(id), &self.slot_user(id), p, Format::Local)
    }

    fn read_slot(&self, id: &str) -> Result<Pair> {
        Ok(Pair { common: read_file(&self.slot_common(id))?, user: read_file(&self.slot_user(id))? })
    }

    fn write_sandbox(&self, id: &str, p: &Pair) -> Result<()> {
        write_slot_files(&self.sandbox_common(id), &self.sandbox_user(id), p, Format::Local)?;
        write_slot_files(&self.sandbox_old_common(id), &self.sandbox_old_user(id), p, Format::Local)
    }

    pub(crate) fn read_sandbox(&self, id: &str) -> Result<Pair> {
        Ok(Pair { common: read_file(&self.sandbox_common(id))?, user: read_file(&self.sandbox_user(id))? })
    }

    /// Writes the game's local files, and the Cloud pair when Steam tracks both.
    fn write_live(&self, p: &Pair) -> Result<()> {
        write_slot_files(&self.local(".sav"), &self.local("_user.sav"), p, Format::Local)?;
        write_slot_files(&self.local(".old"), &self.local("_user.old"), p, Format::Local)?;
        if let Some(r) = &self.remote
            && account::tracked(r, &[&self.cloud_common_name(), &self.cloud_user_name()])
        {
            write_slot_files(&r.join(self.cloud_common_name()), &r.join(self.cloud_user_name()), p, Format::Cloud)?;
        }
        Ok(())
    }

    pub(crate) fn read_entry_name(&self, id: &str) -> Result<Option<String>> {
        Ok(self.read_index()?.saves.into_iter().find(|s| s.id == id).map(|s| s.name))
    }

    fn entry<'a>(&self, idx: &'a Index, id: &str) -> Result<&'a SaveEntry> {
        idx.saves.iter().find(|s| s.id == id).ok_or_else(|| ObsidianError::SaveMissing(id.to_string()))
    }

    fn sandbox_only(&self, idx: &Index, id: &str, op: &str) -> Result<()> {
        match self.entry(idx, id)?.kind {
            SaveKind::Official => Err(ObsidianError::OfficialSave { op: op.to_string() }),
            SaveKind::Sandbox => Ok(()),
        }
    }

    pub(crate) fn require_sandbox(&self, id: &str, op: &str) -> Result<()> {
        let idx = self.read_index()?;
        self.sandbox_only(&idx, id, op)
    }

    fn read_pair(&self, idx: &Index, entry: &SaveEntry) -> Result<Pair> {
        match entry.kind {
            SaveKind::Sandbox => self.read_sandbox(&entry.id),
            SaveKind::Official if idx.active_official.as_deref() == Some(entry.id.as_str()) => {
                self.capture_live().ok_or_else(|| ObsidianError::BadSave("the game's own save files are missing".to_string()))
            }
            SaveKind::Official => self.read_slot(&entry.id),
        }
    }

    fn mod_file(&self, idx: &Index, entry: &SaveEntry) -> PathBuf {
        match entry.kind {
            SaveKind::Sandbox => self.sandbox_mod(&entry.id),
            SaveKind::Official if idx.active_official.as_deref() == Some(entry.id.as_str()) => self.official_mod(),
            SaveKind::Official => self.slot_mod(&entry.id),
        }
    }

    pub fn list(&self) -> Result<SaveList> {
        let idx = self.ensure()?;
        let pointer = read_pointer(&self.data).filter(|(a, _)| *a == self.account).map(|(_, id)| id);
        let active = pointer.clone().or_else(|| idx.active_official.clone());
        let saves = idx
            .saves
            .iter()
            .map(|e| {
                let (stats, error) = match self.read_pair(&idx, e) {
                    Ok(p) => (Some(stats_of(&p)), None),
                    Err(err) => (None, Some(err.to_string())),
                };
                SaveRow {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    kind: e.kind,
                    created: e.created.clone(),
                    last_played: match e.kind {
                        SaveKind::Sandbox => written_at(&self.sandbox_common(&e.id)),
                        SaveKind::Official if Some(&e.id) == idx.active_official.as_ref() => written_at(&self.local(".sav")),
                        SaveKind::Official => None,
                    },
                    snapshot_of: e.snapshot_of.clone(),
                    active: Some(&e.id) == active.as_ref(),
                    live: e.kind == SaveKind::Official && Some(&e.id) == idx.active_official.as_ref(),
                    stats,
                    error,
                }
            })
            .collect();
        Ok(SaveList { account: self.account.clone(), active, saves })
    }

    /// The DLC the account owns, by entitlement id. The game rewrites the `entitlements` map of the
    /// user file at each login from what Steam reports, so the Official save holds the game's own
    /// record; a Sandbox made before the first login goes by the Official save as well. The active
    /// Official save comes first, then the others. No map anywhere means no DLC.
    pub(crate) fn owned_entitlements(&self) -> Result<BTreeSet<String>> {
        let idx = self.ensure()?;
        let mut officials: Vec<&SaveEntry> = idx.saves.iter().filter(|e| e.kind == SaveKind::Official).collect();
        officials.sort_by_key(|e| idx.active_official.as_deref() != Some(e.id.as_str()));
        for entry in officials {
            let Ok(pair) = self.read_pair(&idx, entry) else { continue };
            if let Some(map) = pair.user.root.table("entitlements") {
                return Ok(map.0.iter().filter_map(|(k, _)| k.as_str().map(|s| s.to_string())).collect());
            }
        }
        Ok(BTreeSet::new())
    }

    /// The id the game boots next.
    pub fn active(&self) -> Result<Option<String>> {
        let idx = self.ensure()?;
        Ok(read_pointer(&self.data).filter(|(a, _)| *a == self.account).map(|(_, id)| id).or(idx.active_official))
    }

    /// Selects a save. An Official save goes through the store, backup, restore steps and leaves the
    /// game's own files as that save; a Sandbox is only selected. Refused while the game runs.
    pub fn activate(&self, id: &str) -> Result<()> {
        self.closed()?;
        let mut idx = self.ensure()?;
        let entry = self.entry(&idx, id)?.clone();
        match entry.kind {
            SaveKind::Sandbox => {
                self.read_sandbox(id)?;
                self.set_pointer(Some(id))?;
            }
            SaveKind::Official => {
                if idx.active_official.as_deref() != Some(id) {
                    self.switch_official(&mut idx, id)?;
                }
                self.set_pointer(None)?;
            }
        }
        self.write_settings()
    }

    fn switch_official(&self, idx: &mut Index, id: &str) -> Result<()> {
        let outgoing = self.capture_live();
        if let (Some(active), Some(pair)) = (idx.active_official.clone(), &outgoing) {
            self.write_slot(&active, pair)?;
            copy_or_remove(&self.official_mod(), &self.slot_mod(&active))?;
        }
        crate::backup::backup_game_saves(&self.game, &self.data.join("backups"), KEEP_BACKUPS)?;
        let mut incoming = self.read_slot(id)?;
        strip_queues(&mut incoming);
        if let Some(o) = &outgoing {
            carry_counters(&mut incoming.user.root, &o.user.root);
        }
        self.write_live(&incoming)?;
        copy_or_remove(&self.slot_mod(id), &self.official_mod())?;
        idx.active_official = Some(id.to_string());
        self.save_index(idx)
    }

    /// A new blank Official save. It starts in the library; activate it to play it.
    pub fn create_official(&self, name: &str) -> Result<String> {
        check_name(name)?;
        self.closed()?;
        let mut idx = self.ensure()?;
        let id = self.new_id(&idx);
        self.write_slot(&id, &blank_pair())?;
        idx.saves.push(SaveEntry { id: id.clone(), name: name.trim().to_string(), kind: SaveKind::Official, created: now(), snapshot_of: None });
        self.save_index(&idx)?;
        Ok(id)
    }

    /// Copies an Official save, the live files when it is the active one, into a new Official entry.
    pub fn snapshot(&self, id: &str, name: &str) -> Result<String> {
        check_name(name)?;
        self.closed()?;
        let mut idx = self.ensure()?;
        let entry = self.entry(&idx, id)?.clone();
        if entry.kind != SaveKind::Official {
            return Err(ObsidianError::SandboxSave { op: "snapshot".to_string() });
        }
        let pair = self.read_pair(&idx, &entry)?;
        let new_id = self.new_id(&idx);
        self.write_slot(&new_id, &pair)?;
        copy_or_remove(&self.mod_file(&idx, &entry), &self.slot_mod(&new_id))?;
        idx.saves.push(SaveEntry { id: new_id.clone(), name: name.trim().to_string(), kind: SaveKind::Official, created: now(), snapshot_of: Some(entry.id) });
        self.save_index(&idx)?;
        Ok(new_id)
    }

    /// Every new Sandbox (blank, a copy or an import) starts with all the tutorials done.
    fn add_sandbox(&self, idx: &mut Index, name: &str, pair: &Pair, mod_from: Option<&Path>) -> Result<String> {
        let id = self.new_id(idx);
        let mut pair = pair.clone();
        complete_tutorials(&mut pair);
        self.write_sandbox(&id, &pair)?;
        if let Some(from) = mod_from {
            copy_or_remove(from, &self.sandbox_mod(&id))?;
        }
        idx.saves.push(SaveEntry { id: id.clone(), name: name.trim().to_string(), kind: SaveKind::Sandbox, created: now(), snapshot_of: None });
        self.save_index(idx)?;
        Ok(id)
    }

    /// A new Sandbox from the blank templates.
    pub fn create_sandbox(&self, name: &str) -> Result<String> {
        check_name(name)?;
        self.closed()?;
        let mut idx = self.ensure()?;
        self.add_sandbox(&mut idx, name, &blank_pair(), None)
    }

    /// A new Sandbox from any save, Official or Sandbox. The upload queues stay behind.
    pub fn duplicate(&self, id: &str, name: &str) -> Result<String> {
        check_name(name)?;
        self.closed()?;
        let mut idx = self.ensure()?;
        let entry = self.entry(&idx, id)?.clone();
        let mut pair = self.read_pair(&idx, &entry)?;
        strip_queues(&mut pair);
        let mod_from = self.mod_file(&idx, &entry);
        self.add_sandbox(&mut idx, name, &pair, Some(&mod_from))
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        check_name(name)?;
        self.closed()?;
        let mut idx = self.ensure()?;
        self.sandbox_only(&idx, id, "rename")?;
        if let Some(e) = idx.saves.iter_mut().find(|e| e.id == id) {
            e.name = name.trim().to_string();
        }
        self.save_index(&idx)
    }

    /// Deletes a Sandbox. The one selected can't go; select another first.
    pub fn delete(&self, id: &str) -> Result<()> {
        self.closed()?;
        let mut idx = self.ensure()?;
        self.sandbox_only(&idx, id, "delete")?;
        if read_pointer(&self.data).is_some_and(|(a, p)| a == self.account && p == id) {
            return Err(ObsidianError::SaveInUse);
        }
        for p in [self.sandbox_common(id), self.sandbox_user(id), self.sandbox_old_common(id), self.sandbox_old_user(id), self.sandbox_mod(id)] {
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        idx.saves.retain(|e| e.id != id);
        self.save_index(&idx)
    }

    /// Writes a Sandbox to one zip.
    pub fn export(&self, id: &str, dest: &Path) -> Result<()> {
        use std::io::Write;
        self.closed()?;
        let idx = self.ensure()?;
        self.sandbox_only(&idx, id, "export")?;
        let entry = self.entry(&idx, id)?;
        let pair = self.read_sandbox(id)?;
        let mod_file = self.sandbox_mod(id);
        let tmp = crate::paths::with_suffix(dest, ".obsidian-tmp");
        let written = (|| -> Result<()> {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
            let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let file = |z:&mut zip::ZipWriter<std::fs::File>, name: &str, bytes: &[u8]| -> Result<()> {
                z.start_file(name, opts).map_err(zip_err)?;
                z.write_all(bytes)?;
                Ok(())
            };
            let header = ExportHeader { obsidian_save: 1, name: entry.name.clone() };
            file(&mut z, SAVE_EXPORT_FILE, (serde_json::to_string_pretty(&header)? + "\n").as_bytes())?;
            file(&mut z, "common.sav", &encode_file(&pair.common, Format::Local))?;
            file(&mut z, "user.sav", &encode_file(&pair.user, Format::Local))?;
            if mod_file.is_file() {
                file(&mut z, "mod.obsidian", &std::fs::read(&mod_file)?)?;
            }
            z.finish().map_err(zip_err)?;
            Ok(())
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
            return written;
        }
        crate::paths::rename_retrying(&tmp, dest)?;
        Ok(())
    }

    /// Adds the save in a zip from `export`, always as a Sandbox. The crc is checked, the upload queues
    /// are dropped and paid DLC the account does not own comes out of `item_unlocks`.
    pub fn import(&self, src: &Path, name: Option<&str>) -> Result<String> {
        self.closed()?;
        let mut idx = self.ensure()?;
        let mut z = zip::ZipArchive::new(std::fs::File::open(src)?).map_err(zip_err)?;
        let read_entry = |z: &mut zip::ZipArchive<std::fs::File>, name: &str| -> Result<Vec<u8>> {
            use std::io::Read;
            let f = z.by_name(name).map_err(|_| ObsidianError::NothingToImport(src.display().to_string()))?;
            let mut out = Vec::new();
            f.take(MAX_IMPORT).read_to_end(&mut out)?;
            Ok(out)
        };
        let header: ExportHeader = serde_json::from_slice(&read_entry(&mut z, SAVE_EXPORT_FILE)?)?;
        if header.obsidian_save > 1 {
            return Err(ObsidianError::NewerSaveExport(src.display().to_string()));
        }
        let name = name.map(str::to_string).unwrap_or(header.name);
        check_name(&name)?;
        let mut pair = Pair { common: decode_file(&read_entry(&mut z, "common.sav")?)?, user: decode_file(&read_entry(&mut z, "user.sav")?)? };
        strip_queues(&mut pair);
        remove_unowned_dlc(&mut pair.user.root, &self.owned_entitlements()?);
        let mod_data = read_entry(&mut z, "mod.obsidian").ok();
        let id = self.add_sandbox(&mut idx, &name, &pair, None)?;
        if let Some(bytes) = mod_data {
            write_atomic(&self.sandbox_mod(&id), &bytes)?;
        }
        Ok(id)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportHeader {
    obsidian_save: u32,
    name: String,
}

fn zip_err(e: impl std::fmt::Display) -> ObsidianError {
    ObsidianError::UnsupportedArchive(e.to_string())
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

/// When a file was last written, in the format of `now`.
fn written_at(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(chrono::DateTime::<chrono::Local>::from(modified).to_rfc3339())
}

/// What Play does before launching: refuses a Sandbox the game could not open. (Profiles naming a
/// save to play were archived under the git tag `archive/profile-save`.)
pub fn prepare_play(data: &Path, game: &GameInstall, running: fn() -> bool) -> Result<()> {
    prepare_with(data, game, || Saves::open(data, game, running))
}

fn prepare_with(data: &Path, game: &GameInstall, open: impl Fn() -> Result<Saves>) -> Result<()> {
    if read_pointer(data).is_some() {
        // Clears a Sandbox left selected by another Steam account.
        let _ = open();
    }
    if active_sandbox(data).is_some() && !crate::deploy::loader_deployed(game, data) {
        return Err(ObsidianError::SandboxNeedsDeploy);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    pub const ACCOUNT: &str = "107296691";
    pub const UID: &str = "1100001066537b3";

    pub struct World {
        pub _t: tempfile::TempDir,
        pub data: PathBuf,
        pub game: GameInstall,
        pub remote: PathBuf,
    }

    impl World {
        pub fn saves(&self) -> Saves {
            Saves::for_account(&self.data, &self.game, ACCOUNT, || false)
        }
        pub fn local_common(&self) -> PathBuf {
            self.game.saves_dir.join(format!("helldivers_steam_{UID}.sav"))
        }
        pub fn local_user(&self) -> PathBuf {
            self.game.saves_dir.join(format!("helldivers_steam_{UID}_user.sav"))
        }
    }

    /// A user save with the given XP and a counter, plus a queue, to tell saves apart.
    pub fn user_save(xp: f64, counter: f64) -> SaveFile {
        let mut f = SaveFile::empty();
        f.root.set("player_xp", Value::Number(xp));
        f.root.set("backend_planet_index_win32", Value::Number(counter));
        f.root.table_mut("leaderboard_stats_to_upload").set_strings(&["q".to_string()]);
        f
    }

    pub fn common_save(flag: bool) -> SaveFile {
        let mut f = SaveFile::empty();
        f.root.set("show_hints", Value::Bool(flag as u32));
        f.root.table_mut("mission_stats_to_upload").set_strings(&["q".to_string()]);
        f
    }

    /// A game with one account's local files, and a Cloud folder Steam tracks.
    pub fn world() -> World {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let saves_dir = t.path().join("appdata/saves");
        let remote = t.path().join("steam/userdata").join(ACCOUNT).join("394510/remote");
        std::fs::create_dir_all(&remote).unwrap();
        let game = GameInstall { root: t.path().join("game"), saves_dir, cloud_saves: vec![remote.clone()] };
        let w = World { _t: t, data, game, remote };
        write_verified(&w.local_common(), &common_save(false), Format::Local).unwrap();
        write_verified(&w.local_user(), &user_save(1000.0, 5.0), Format::Local).unwrap();
        w
    }

    /// Writes the entitlements the game would, with the names as the keys, into the live user file.
    pub fn own_dlc(w: &World, entitlements: &[&str]) {
        let mut user = read_file(&w.local_user()).unwrap();
        let map = user.root.table_mut("entitlements");
        for e in entitlements {
            map.table_mut(e).set("id", Value::text(e));
        }
        write_verified(&w.local_user(), &user, Format::Local).unwrap();
    }

    pub fn track(w: &World) {
        let names = format!("\"394510\" {{ \"helldivers_save\" {{ }} \"helldivers_save{UID}\" {{ }} }}");
        put(w.remote.parent().unwrap(), "remotecache.vdf", names.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    fn xp_of(p: &Pair) -> f64 {
        p.user.root.number("player_xp").unwrap()
    }

    #[test]
    fn the_first_use_adopts_the_game_files_as_an_official_save() {
        let w = world();
        let list = w.saves().list().unwrap();
        assert_eq!(list.saves.len(), 1);
        let row = &list.saves[0];
        assert_eq!(row.kind, SaveKind::Official);
        assert!(row.active && row.live);
        assert_eq!(row.stats.as_ref().unwrap().xp, 1000.0);
        assert_eq!(w.saves().list().unwrap().saves.len(), 1, "adopted once");
    }

    #[test]
    fn a_save_for_another_account_stays_apart() {
        let w = world();
        w.saves().list().unwrap();
        let other = Saves::for_account(&w.data, &w.game, "55", || false);
        assert!(other.list().unwrap().saves.is_empty(), "the other account's files aren't there");
        assert_eq!(w.saves().list().unwrap().saves.len(), 1);
    }

    #[test]
    fn activating_official_strips_queues_and_carries_higher_counters() {
        let w = world();
        track(&w);
        let s = w.saves();
        let list = s.list().unwrap();
        let first = list.saves[0].id.clone();
        let snap = s.snapshot(&first, "Snap").unwrap();

        // The player goes on: the game's files move ahead, with a higher counter.
        write_verified(&w.local_user(), &user_save(2000.0, 9.0), Format::Local).unwrap();
        s.activate(&snap).unwrap();

        // The outgoing save went to the library.
        assert_eq!(xp_of(&s.read_slot(&first).unwrap()), 2000.0);

        // The restored save is live, with no queues and the higher counter.
        let live_user = read_file(&w.local_user()).unwrap();
        assert_eq!(live_user.root.number("player_xp"), Some(1000.0));
        assert_eq!(live_user.root.number("backend_planet_index_win32"), Some(9.0));
        assert!(!live_user.root.contains("leaderboard_stats_to_upload"));
        assert!(!read_file(&w.local_common()).unwrap().root.contains("mission_stats_to_upload"));
        assert!(w.game.saves_dir.join(format!("helldivers_steam_{UID}.old")).is_file());

        // Steam tracks the files, so the Cloud pair went out too, in Cloud format.
        let cloud = std::fs::read(w.remote.join("helldivers_save")).unwrap();
        assert!(cloud.starts_with(&[0xd4, 0xfd]));
        assert_eq!(decode_file(&cloud).unwrap().root, read_file(&w.local_common()).unwrap().root);
        assert_eq!(s.list().unwrap().active, Some(snap.clone()));

        assert_eq!(crate::backup::list_backups(&w.data.join("backups")).len(), 1);
    }

    #[test]
    fn with_no_tracking_only_the_local_files_are_written() {
        let w = world();
        let s = w.saves();
        let list = s.list().unwrap();
        let snap = s.snapshot(&list.saves[0].id, "Snap").unwrap();
        s.activate(&snap).unwrap();
        assert!(!w.remote.join("helldivers_save").exists());
    }

    #[test]
    fn the_newest_of_cloud_and_local_is_what_counts() {
        let w = world();
        track(&w);
        let s = w.saves();
        s.list().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        write_verified(&w.remote.join(format!("helldivers_save{UID}")), &user_save(7777.0, 1.0), Format::Cloud).unwrap();
        assert_eq!(xp_of(&s.capture_live().unwrap()), 7777.0);
    }

    #[test]
    fn sandbox_only_actions_are_refused_on_official() {
        let w = world();
        let s = w.saves();
        let official = s.list().unwrap().saves[0].id.clone();
        let off = |e: Result<()>| assert!(matches!(e, Err(ObsidianError::OfficialSave { .. })));
        off(s.rename(&official, "x"));
        off(s.delete(&official));
        off(s.export(&official, &w.data.join("x.zip")));
    }

    #[test]
    fn a_running_game_refuses_every_change() {
        let w = world();
        w.saves().list().unwrap();
        let busy = Saves::for_account(&w.data, &w.game, ACCOUNT, || true);
        let official = busy.list().unwrap().saves[0].id.clone();
        assert!(matches!(busy.activate(&official), Err(ObsidianError::GameRunning)));
        assert!(matches!(busy.create_sandbox("a"), Err(ObsidianError::GameRunning)));
        assert!(matches!(busy.delete(&official), Err(ObsidianError::GameRunning)));
    }

    #[test]
    fn sandbox_create_rename_duplicate_delete() {
        let w = world();
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("Mine").unwrap();
        assert_eq!(id.len(), 8);
        assert!(s.sandbox_common(&id).is_file() && s.sandbox_user(&id).is_file());
        s.rename(&id, "Renamed").unwrap();
        let official = s.list().unwrap().saves[0].id.clone();
        let copy = s.duplicate(&official, "From official").unwrap();
        let p = s.read_sandbox(&copy).unwrap();
        assert_eq!(p.user.root.number("player_xp"), Some(1000.0));
        assert!(!p.user.root.contains("leaderboard_stats_to_upload"), "duplicating keeps the queue out");
        s.delete(&id).unwrap();
        assert!(!s.sandbox_common(&id).exists());
        assert!(matches!(s.rename("nope", "x"), Err(ObsidianError::SaveMissing(_))));
    }

    #[test]
    fn a_new_sandbox_starts_with_every_tutorial_done() {
        let w = world();
        let s = w.saves();
        let official = s.list().unwrap().saves[0].id.clone();
        let done = |id: &str| {
            let p = s.read_sandbox(id).unwrap();
            TUTORIALS.iter().all(|k| p.common.root.get(k).and_then(Value::as_bool) == Some(true))
        };
        assert!(!TUTORIALS.iter().all(|k| blank_pair().common.root.get(k).and_then(Value::as_bool) == Some(true)), "the blank save has them open");
        assert!(done(&s.create_sandbox("Blank").unwrap()));
        assert!(done(&s.duplicate(&official, "Copy").unwrap()));
    }

    #[test]
    fn selecting_a_sandbox_writes_the_settings_line_and_official_clears_it() {
        let w = world();
        let s = w.saves();
        let official = s.list().unwrap().saves[0].id.clone();
        let id = s.create_sandbox("Mine").unwrap();
        s.activate(&id).unwrap();
        let settings = || std::fs::read_to_string(crate::settings::game_side_dir(&w.game.saves_dir).join(crate::settings::FILE)).unwrap();
        assert!(settings().contains(&format!("sandbox\t{id}\n")));
        assert!(settings().contains("block_achievements\t1\n"), "forced while a Sandbox is selected");
        assert_eq!(active_sandbox(&w.data).as_deref(), Some(id.as_str()));
        assert_eq!(s.list().unwrap().active.as_deref(), Some(id.as_str()));
        assert!(matches!(s.delete(&id), Err(ObsidianError::SaveInUse)));
        s.activate(&official).unwrap();
        assert!(!settings().contains("sandbox\t"));
        assert_eq!(active_sandbox(&w.data), None);
        // The Sandbox stays as it was, and the Official files were not touched by it.
        assert_eq!(xp_of(&s.capture_live().unwrap()), 1000.0);
    }

    #[test]
    fn export_import_is_always_sandbox_and_clean() {
        let w = world();
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("Trip").unwrap();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.table_mut("item_unlocks").set_strings(&["weapon_plasmarifle".to_string(), "stratagem_napalmstrike".to_string(), "mod_item".to_string()]);
        p.user.root.table_mut("leaderboard_stats").set_strings(&["q".to_string()]);
        s.write_sandbox(&id, &p).unwrap();
        std::fs::write(s.sandbox_mod(&id), "a.b\tk\ts\tv\n").unwrap();

        let zip = w.data.join("trip.zip");
        s.export(&id, &zip).unwrap();

        let imported = s.import(&zip, None).unwrap();
        let row = s.list().unwrap().saves.into_iter().find(|r| r.id == imported).unwrap();
        assert_eq!((row.kind, row.name.as_str()), (SaveKind::Sandbox, "Trip"));
        let got = s.read_sandbox(&imported).unwrap();
        assert_eq!(got.user.root.table("item_unlocks").unwrap().strings(), ["stratagem_napalmstrike", "mod_item"], "paid DLC removed, unknown names kept");
        assert!(!got.user.root.contains("leaderboard_stats"));
        assert_eq!(std::fs::read_to_string(s.sandbox_mod(&imported)).unwrap(), "a.b\tk\ts\tv\n");

        // A bad crc inside the zip is refused and adds nothing.
        let bad = w.data.join("bad.zip");
        {
            use std::io::Write;
            let mut z = zip::ZipWriter::new(std::fs::File::create(&bad).unwrap());
            let opts = zip::write::SimpleFileOptions::default();
            z.start_file(SAVE_EXPORT_FILE, opts).unwrap();
            z.write_all(br#"{"obsidianSave":1,"name":"x"}"#).unwrap();
            for f in ["common.sav", "user.sav"] {
                z.start_file(f, opts).unwrap();
                z.write_all(&[1u8; 40]).unwrap();
            }
            z.finish().unwrap();
        }
        let before = s.list().unwrap().saves.len();
        assert!(matches!(s.import(&bad, None), Err(ObsidianError::BadSave(_))));
        assert_eq!(s.list().unwrap().saves.len(), before);
    }

    #[test]
    fn import_keeps_the_dlc_the_account_owns() {
        let w = world();
        own_dlc(&w, &["ENT011"]);
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("Trip").unwrap();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.table_mut("item_unlocks").set_strings(&["weapon_plasmarifle".to_string(), "stratagem_uav_drone".to_string()]);
        s.write_sandbox(&id, &p).unwrap();
        let zip = w.data.join("trip.zip");
        s.export(&id, &zip).unwrap();
        let got = s.read_sandbox(&s.import(&zip, None).unwrap()).unwrap();
        assert_eq!(got.user.root.table("item_unlocks").unwrap().strings(), ["weapon_plasmarifle"], "ENT011 stays, ENT001 goes");
    }

    #[test]
    fn ownership_comes_from_the_official_save_and_stays_empty_without_one() {
        let w = world();
        let s = w.saves();
        assert!(s.owned_entitlements().unwrap().is_empty(), "no entitlements in the save");
        own_dlc(&w, &["ENT001", "ENT003"]);
        assert_eq!(s.owned_entitlements().unwrap(), ["ENT001".to_string(), "ENT003".to_string()].into());
        let list = s.list().unwrap();
        // A snapshot of a save from the first login on keeps the map: the older of two Official saves can answer.
        let snap = s.snapshot(&list.saves[0].id, "Snap").unwrap();
        s.activate(&snap).unwrap();
        write_verified(&w.local_user(), &user_save(1.0, 1.0), Format::Local).unwrap();
        let stored: Vec<_> = s.list().unwrap().saves.iter().map(|r| r.id.clone()).collect();
        assert_eq!(stored.len(), 2);
        assert_eq!(s.owned_entitlements().unwrap(), ["ENT001".to_string(), "ENT003".to_string()].into(), "the blank live save has no map");
    }

    #[test]
    fn dlc_removal_keeps_order_and_renumbers() {
        let mut t = Table::default();
        t.table_mut("item_unlocks").set_strings(&["a_unknown".to_string(), "weapon_plasmarifle".to_string(), "b_unknown".to_string()]);
        remove_unowned_dlc(&mut t, &BTreeSet::new());
        let list = t.table("item_unlocks").unwrap();
        assert_eq!(list.at(1).and_then(Value::as_str).as_deref(), Some("a_unknown"));
        assert_eq!(list.at(2).and_then(Value::as_str).as_deref(), Some("b_unknown"));
        assert_eq!(list.0.len(), 2);
    }

    #[test]
    fn counters_need_the_backend_shape() {
        assert!(is_counter("backend_planet_index_win32"));
        assert!(is_counter("backend_leaderboard_provinggrounds_stats_index_linux"));
        assert!(!is_counter("backend_other_index_win32"));
        assert!(!is_counter("planet_index"));
    }

    #[test]
    fn a_blank_official_can_be_created_and_activated() {
        let w = world();
        let s = w.saves();
        s.list().unwrap();
        let blank = s.create_official("Fresh").unwrap();
        s.activate(&blank).unwrap();
        let live = read_file(&w.local_common()).unwrap();
        assert!(live.root.contains("show_hints"), "the template");
    }

    #[test]
    fn play_refuses_an_undeployed_sandbox_and_boots_an_official_save() {
        let w = world();
        let s = w.saves();
        let official = s.list().unwrap().saves[0].id.clone();
        let id = s.create_sandbox("Mine").unwrap();
        let open = || Ok(w.saves());
        prepare_with(&w.data, &w.game, open).unwrap();
        s.activate(&id).unwrap();
        assert!(matches!(prepare_with(&w.data, &w.game, open), Err(ObsidianError::SandboxNeedsDeploy)));
        assert_eq!(active_sandbox(&w.data).as_deref(), Some(id.as_str()), "the selection stays");
        s.activate(&official).unwrap();
        prepare_with(&w.data, &w.game, open).unwrap();
    }

    #[test]
    fn an_old_profile_naming_a_save_still_loads() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("profile.json"), br#"{"mods":[],"save":"abc123"}"#).unwrap();
        let p: Profile = serde_json::from_slice(&std::fs::read(t.path().join("profile.json")).unwrap()).unwrap();
        assert!(p.mods.is_empty());
    }

    #[test]
    fn a_failed_read_shows_as_a_row_error_not_a_failed_list() {
        let w = world();
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("Mine").unwrap();
        std::fs::write(s.sandbox_common(&id), b"junk").unwrap();
        let row = s.list().unwrap().saves.into_iter().find(|r| r.id == id).unwrap();
        assert!(row.stats.is_none() && row.error.is_some());
    }

    #[test]
    fn bad_names_are_refused() {
        let w = world();
        let s = w.saves();
        s.list().unwrap();
        assert!(matches!(s.create_sandbox("  "), Err(ObsidianError::SaveName(_))));
        assert!(matches!(s.create_sandbox("a\nb"), Err(ObsidianError::SaveName(_))));
    }
}
