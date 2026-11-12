//! Commander: custom Galactic Wars. The Obsidian side of it (the contract is the ObsidianCommander
//! `docs/spec-m1.md`):
//!
//! - which wars this PC plays, and which one is selected (`state.json` in `<data>/commander`);
//! - the Commander package: installed from a ZIP into `<data>/commander/package`, run as a separate
//!   server process by the app;
//! - the relay ([`relay`]): the game only ever talks to it, on loopback, and it carries the war
//!   requests to the war server without the Steam ticket;
//! - the war line of the settings file, which turns on the loader war mode;
//! - finding wars on the local network ([`lan`]);
//! - removing everything Commander left on this PC ([`wipe`]).
//!
//! A war is only played on a Sandbox save, so nothing from it reaches the Official save, and the
//! loader refuses war mode without one. Any Sandbox plays it, unless the war says `own_save_only`:
//! then only the Sandbox made for it does. A war gets that save on first use when the player picks
//! none.

pub mod client;
pub mod firewall;
pub mod lan;
pub mod package;
pub mod relay;
pub mod server;
pub mod wipe;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths::write_atomic;
use crate::{ObsidianError, Result};

const STATE: &str = "state.json";
/// The region names the loader writes in the game-side folder.
pub const NAMES_FILE: &str = "commander-names.txt";
/// The protocol of `docs/spec-m1.md` this Obsidian speaks.
pub const PROTOCOL: u32 = 1;
/// The companion mod every player of an Enhanced war runs (github.com/Alomare/HD1-EnhancedWarClient).
pub const ENHANCED_MOD: &str = "alomare.enhancedwar";
/// The war model that plays through [`ENHANCED_MOD`]. Vanilla wars carry no model.
pub const ENHANCED_MODEL: &str = "enhanced";

/// Where a war runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarHost {
    /// On the Commander server on this PC.
    #[default]
    Local,
    /// On another PC of the local network.
    Lan,
}

/// A war this PC plays.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarRef {
    pub id: String,
    pub name: String,
    pub host: WarHost,
    /// For a LAN war: `http://<address>:<port>`. A local war address follows the server port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// The war Sandbox save, once made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save: Option<String>,
    /// The computer that hosts a LAN war, as it announced itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer: Option<String>,
    /// "enhanced" for a war that plays through the Enhanced War Client mod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The invite code of a war joined from outside the host network. The relay gives it to the
    /// war server with every request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The war settings of a LAN war, as the host announced them when the war was selected. A war
    /// hosted here is read from the war file instead, so a change by the host counts at once.
    #[serde(default, skip_serializing_if = "is_false")]
    pub own_save_only: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_achievements: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The war settings Obsidian acts on (`war.settings` of the war file, the war summaries of
/// `/api/info` and the LAN announcement).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct WarFlags {
    /// Only the Sandbox made for the war plays it.
    #[serde(default)]
    pub own_save_only: bool,
    /// Steam achievements are not blocked while the war is played. The player's own guardrail
    /// still applies.
    #[serde(default)]
    pub allow_achievements: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub version: String,
    pub protocol: u32,
    /// The server file name inside the package folder.
    pub server: String,
    /// The interface folder inside the package folder.
    pub ui: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<Installed>,
    #[serde(default)]
    pub wars: Vec<WarRef>,
    /// The war the next launch plays. None: the Official war.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playing: Option<String>,
    /// The relay port. Chosen once, so the settings file a game already read stays right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay_port: Option<u16>,
    /// The secret in the relay path. Changed each time a war is selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// The local server port. Chosen once, so local war addresses stay the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_port: Option<u16>,
}

pub fn dir(data: &Path) -> PathBuf {
    data.join("commander")
}

impl State {
    pub fn load(data: &Path) -> State {
        let path = dir(data).join(STATE);
        let Ok(text) = std::fs::read_to_string(&path) else { return State::default() };
        match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(_) => {
                // A file that no longer reads is kept beside the new one, so the war list in it
                // can still be recovered by hand.
                let _ = std::fs::copy(&path, path.with_extension("json.bad"));
                State::default()
            }
        }
    }

    pub fn save(&self, data: &Path) -> Result<()> {
        let d = dir(data);
        std::fs::create_dir_all(&d)?;
        write_atomic(&d.join(STATE), serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    pub fn war(&self, id: &str) -> Option<&WarRef> {
        self.wars.iter().find(|w| w.id == id)
    }

    /// Adds a war or updates the one with that id, keeping the save, and the invite code when the
    /// new entry brings none (a war picked again from a list that never held the code).
    pub fn upsert(&mut self, war: WarRef) {
        match self.wars.iter_mut().find(|w| w.id == war.id) {
            Some(w) => {
                let (save, key) = (w.save.take(), w.key.take());
                *w = WarRef { save: war.save.clone().or(save), key: war.key.clone().or(key), ..war };
            }
            None => self.wars.push(war),
        }
    }

    /// Where the relay sends the game requests for `war`: the war API.
    pub fn war_api(&self, war: &WarRef) -> Option<String> {
        let base = match war.host {
            WarHost::Local => format!("http://127.0.0.1:{}", self.server_port?),
            WarHost::Lan => war.base.clone()?,
        };
        Some(format!("{}/w/{}/1.0/", base.trim_end_matches('/'), war.id))
    }

    /// The war being played, with where the requests go.
    pub fn route(&self) -> Option<relay::Route> {
        let war = self.war(self.playing.as_deref()?)?;
        Some(relay::Route { token: self.token.clone()?, upstream: self.war_api(war)?, key: war.key.clone() })
    }
}

/// The war line of the settings file, for the war being played: only with a Sandbox save selected
/// that the war accepts, which the caller checks against the active Sandbox.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WarLine {
    pub id: String,
    /// Where the game sends war requests: the relay, with the launch token.
    pub url: String,
    /// The tag that keeps the players of this war in a separate matchmaking pool.
    pub pool: String,
    /// The war allows Steam achievements: the Sandbox block stays off for it (the
    /// `war_achievements` line of the settings file).
    pub achievements: bool,
    /// "enhanced" for an Enhanced war (the `war_model` line), for the Enhanced War Client mod
    /// through `obsidian.war()`. None for a Vanilla war.
    pub model: Option<String>,
}

/// What a war file in the server folder says, of what Obsidian needs.
#[derive(Deserialize)]
struct WarFile {
    id: String,
    name: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    settings: WarFlags,
}

fn war_file(data: &Path, id: &str) -> Option<WarFile> {
    let text = std::fs::read_to_string(server::server_data(data).join("wars").join(id).join("war.json")).ok()?;
    serde_json::from_str::<WarFile>(&text).ok().filter(|w| w.id == id)
}

/// The settings of a war: from the war file for a war hosted here, else as the host announced
/// them. A local war whose file no longer reads keeps the strict defaults.
pub fn war_flags(data: &Path, war: &WarRef) -> WarFlags {
    match war.host {
        WarHost::Local if valid_pick(war) => war_file(data, &war.id).map(|f| f.settings).unwrap_or_default(),
        WarHost::Local => WarFlags::default(),
        WarHost::Lan => WarFlags { own_save_only: war.own_save_only, allow_achievements: war.allow_achievements },
    }
}

/// A LAN war with the settings its server states now, asked at `<base>/api/info`. When the server
/// does not answer, the war keeps the settings it was picked with (from the announcement).
pub fn with_host_flags(war: WarRef) -> WarRef {
    if war.host != WarHost::Lan {
        return war;
    }
    match war.base.as_deref().and_then(|b| lan::flags_at(b, war.key.as_deref(), &war.id)) {
        Some(f) => WarRef { own_save_only: f.own_save_only, allow_achievements: f.allow_achievements, ..war },
        None => war,
    }
}

/// Whether `war` may be played on the Sandbox `save`: any Sandbox, or only the one made for it when
/// the war says `own_save_only`.
pub fn save_allowed(war: &WarRef, flags: WarFlags, save: &str) -> bool {
    !flags.own_save_only || war.save.as_deref() == Some(save)
}

/// The war line, when a war is selected and `sandbox` is a Sandbox the war accepts.
pub fn war_line(data: &Path, sandbox: Option<&str>) -> Option<WarLine> {
    let sandbox = sandbox?;
    let s = State::load(data);
    let war = s.war(s.playing.as_deref()?)?;
    let flags = war_flags(data, war);
    if !save_allowed(war, flags, sandbox) {
        return None;
    }
    // A war hosted here: the model of the war file. A LAN war: the model it was picked with.
    let model = match war.host {
        WarHost::Local => war_file(data, &war.id).map(|f| known_model(f.model)).unwrap_or_else(|| known_model(war.model.clone())),
        WarHost::Lan => known_model(war.model.clone()),
    };
    Some(WarLine {
        id: war.id.clone(),
        url: format!("http://127.0.0.1:{}/{}/1.0/", s.relay_port?, s.token.as_deref()?),
        pool: war_pool_tag(&war.id),
        achievements: flags.allow_achievements,
        model,
    })
}

/// The war the next launch plays on the selected Sandbox, if any: the war selected, when it
/// accepts that save. None means the official war.
pub fn playing_now(data: &Path, sandbox: Option<&str>) -> Option<String> {
    war_line(data, sandbox).map(|l| l.id)
}

/// The pool tag of a war: players of one war find each other and nobody else.
pub fn war_pool_tag(id: &str) -> String {
    use sha2::{Digest, Sha256};
    let d = Sha256::digest(format!("obsidian:war:1\n{id}").as_bytes());
    d.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// A fresh random hex string from the system random source.
pub fn random_token(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    if getrandom::fill(&mut raw).is_ok() {
        return raw.iter().map(|b| format!("{b:02x}")).collect();
    }
    // The system source failed, which Windows and Linux do not do in practice: hashed bits instead.
    let mut out = String::new();
    // RandomState is seeded from the OS; hashing a counter through fresh ones gives unpredictable bits.
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut i = 0u64;
    while out.len() < bytes * 2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        out.push_str(&format!("{:016x}", h.finish()));
        i += 1;
    }
    out.truncate(bytes * 2);
    out
}

/// A free loopback port, found by binding port 0.
pub fn free_port() -> Result<u16> {
    Ok(std::net::TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port())
}

/// The relay port, chosen and stored on first use.
pub fn ensure_relay_port(data: &Path) -> Result<u16> {
    let mut s = State::load(data);
    if let Some(p) = s.relay_port {
        return Ok(p);
    }
    let p = free_port()?;
    s.relay_port = Some(p);
    s.save(data)?;
    Ok(p)
}

/// Selects `id` for play with a new launch token. The caller then selects the war save, which
/// writes the settings file with the war line.
pub fn select(data: &Path, id: Option<&str>) -> Result<State> {
    let mut s = State::load(data);
    if let Some(id) = id {
        if s.war(id).is_none() {
            return Err(ObsidianError::WarUnknown(id.to_string()));
        }
        if s.relay_port.is_none() {
            s.relay_port = Some(free_port()?);
        }
        s.token = Some(random_token(16));
    }
    s.playing = id.map(str::to_string);
    s.save(data)?;
    Ok(s)
}

/// Records the Sandbox save made for a war.
pub fn set_save(data: &Path, id: &str, save: &str) -> Result<()> {
    let mut s = State::load(data);
    let w = s.wars.iter_mut().find(|w| w.id == id).ok_or_else(|| ObsidianError::WarUnknown(id.to_string()))?;
    w.save = Some(save.to_string());
    s.save(data)
}

/// Forgets a war on this PC. The save stays in Saves.
pub fn forget(data: &Path, id: &str) -> Result<State> {
    let mut s = State::load(data);
    s.wars.retain(|w| w.id != id);
    if s.playing.as_deref() == Some(id) {
        s.playing = None;
    }
    s.save(data)?;
    Ok(s)
}

/// A save name from a war name: Saves takes 1 to 40 characters.
pub fn save_name(war: &str) -> String {
    let clean: String = war.chars().filter(|c| !c.is_control()).collect();
    let clean = clean.trim();
    let base = if clean.is_empty() { "Custom War" } else { clean };
    base.chars().take(40).collect()
}

/// Whether a war as named from outside (the war screens, the LAN list, the CLI) can be used: a plain
/// id, and for a LAN war `http://<address>:<port>` with nothing after it.
pub fn valid_pick(war: &WarRef) -> bool {
    let id_ok = !war.id.is_empty() && war.id.len() <= 32 && war.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    let base_ok = match (war.host, &war.base) {
        (WarHost::Local, _) => true,
        (WarHost::Lan, Some(b)) => b.len() < 120 && b.strip_prefix("http://").is_some_and(|rest| !rest.is_empty() && !rest.contains(['/', '?', '#', '@', ' '])),
        (WarHost::Lan, None) => false,
    };
    // An invite code is letters, digits and dashes, as the war server writes it.
    let key_ok = war.key.as_deref().is_none_or(|k| !k.is_empty() && k.len() <= 32 && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    id_ok && base_ok && key_ok
}

/// Selects a war for the next launch on `save`, a Sandbox the war accepts, or with none on the
/// Sandbox made for the war (made on first use), and selects that save, which writes the settings
/// file with the war line. An Official save never plays a custom war. A local war needs the server
/// port known, so start the server first. Returns the save selected.
pub fn play_war(data: &Path, saves: &crate::saves::Saves, war: WarRef, save: Option<&str>) -> Result<String> {
    if !valid_pick(&war) {
        return Err(ObsidianError::WarUnknown(war.id));
    }
    let id = war.id.clone();
    let name = war.name.clone();
    let known: Vec<String> = saves.list()?.saves.into_iter().filter(|r| r.kind == crate::saves::SaveKind::Sandbox).map(|r| r.id).collect();
    if let Some(s) = save
        && !known.iter().any(|k| k == s)
    {
        return Err(ObsidianError::WarSave(format!("{s} is not a Sandbox save")));
    }
    let mut state = State::load(data);
    state.upsert(WarRef { name: war.name.chars().take(60).collect(), save: None, ..war });
    state.save(data)?;
    let entry = State::load(data).war(&id).cloned().ok_or_else(|| ObsidianError::WarUnknown(id.clone()))?;
    let own = entry.save.clone().filter(|s| known.contains(s));
    let save = match (save, own) {
        (Some(s), _) => {
            if !save_allowed(&entry, war_flags(data, &entry), s) {
                return Err(ObsidianError::WarSave("this war is played only on the Sandbox save made for it".into()));
            }
            s.to_string()
        }
        (None, Some(s)) => s,
        (None, None) => {
            let s = saves.create_sandbox(&save_name(&name))?;
            set_save(data, &id, &s)?;
            s
        }
    };
    select(data, Some(&id))?;
    saves.activate(&save)?;
    Ok(save)
}

/// Back to the official war: no war selected, and the Official save that's in the game files.
pub fn play_official(data: &Path, saves: &crate::saves::Saves) -> Result<()> {
    select(data, None)?;
    let list = saves.list()?;
    let official = list
        .saves
        .iter()
        .find(|r| r.kind == crate::saves::SaveKind::Official && r.live)
        .or_else(|| list.saves.iter().find(|r| r.kind == crate::saves::SaveKind::Official));
    match official {
        Some(row) => saves.activate(&row.id),
        None => Ok(()),
    }
}

/// Back to the official war on a save of the player's choice, as the Play choice picks it.
pub fn play_vanilla(data: &Path, saves: &crate::saves::Saves, save: &str) -> Result<()> {
    select(data, None)?;
    saves.activate(save)
}

/// A war the next launch can play, for the Play choice.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarChoice {
    pub id: String,
    pub name: String,
    pub host: WarHost,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computer: Option<String>,
    /// The war Sandbox save, once made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save: Option<String>,
    /// "enhanced" for an Enhanced war.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// "running" or "paused" for a war hosted here. A LAN war has none: the host PC knows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Only the war Sandbox plays this war.
    pub own_save_only: bool,
    /// The war allows Steam achievements.
    pub allow_achievements: bool,
}

/// The model of a war as Obsidian acts on it: "enhanced", or none for a Vanilla war. Any other
/// model, such as the archived "evolved", plays as a Vanilla war.
pub fn known_model(model: Option<String>) -> Option<String> {
    model.filter(|m| m == ENHANCED_MODEL)
}

/// The wars the next launch can play: the wars hosted here that run or are paused (with the package
/// installed), then the LAN wars this PC joined. The war selected now is always there, whatever
/// state it is in. The hosted wars come from the server folder, so the server needn't run.
pub fn war_choices(data: &Path) -> Vec<WarChoice> {
    let state = State::load(data);
    let playing = state.playing.as_deref();
    let mut hosted: Vec<WarChoice> = Vec::new();
    if package::installed(data).is_some()
        && let Ok(rd) = std::fs::read_dir(server::server_data(data).join("wars"))
    {
        for e in rd.flatten() {
            let Some(w) = std::fs::read_to_string(e.path().join("war.json")).ok().and_then(|t| serde_json::from_str::<WarFile>(&t).ok()) else { continue };
            let live = matches!(w.state.as_deref(), Some("running" | "paused"));
            let pick = WarRef { id: w.id.clone(), host: WarHost::Local, ..Default::default() };
            if !valid_pick(&pick) || (!live && playing != Some(w.id.as_str())) {
                continue;
            }
            let save = state.war(&w.id).filter(|r| r.host == WarHost::Local).and_then(|r| r.save.clone());
            let name: String = w.name.chars().take(60).collect();
            hosted.push(WarChoice {
                id: w.id,
                name,
                host: WarHost::Local,
                base: None,
                computer: None,
                save,
                model: known_model(w.model),
                state: w.state,
                own_save_only: w.settings.own_save_only,
                allow_achievements: w.settings.allow_achievements,
            });
        }
    }
    hosted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
    let lan = state.wars.iter().filter(|w| w.host == WarHost::Lan).map(|w| WarChoice {
        id: w.id.clone(),
        name: w.name.clone(),
        host: WarHost::Lan,
        base: w.base.clone(),
        computer: w.computer.clone(),
        save: w.save.clone(),
        model: known_model(w.model.clone()),
        state: None,
        own_save_only: w.own_save_only,
        allow_achievements: w.allow_achievements,
    });
    hosted.into_iter().chain(lan).collect()
}

/// The names of the game regions in the player language, as the loader wrote them the last
/// time the game started in war mode: `key<TAB>words` lines.
pub fn region_names(game_side: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(game_side.join(NAMES_FILE)) else { return Vec::new() };
    text.lines().filter_map(|l| l.split_once('\t')).filter(|(k, v)| !k.is_empty() && !v.is_empty()).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lan_war() -> WarRef {
        WarRef { id: "k3x9q2".into(), name: "Night War".into(), host: WarHost::Lan, base: Some("http://192.168.0.7:17494/".into()), save: None, computer: Some("DESK".into()), model: None, key: None, ..Default::default() }
    }

    #[test]
    fn state_round_trips_and_routes_a_lan_war() {
        let t = tempfile::tempdir().unwrap();
        let mut s = State::default();
        s.upsert(lan_war());
        s.save(t.path()).unwrap();
        let s = select(t.path(), Some("k3x9q2")).unwrap();
        let r = s.route().unwrap();
        assert_eq!(r.upstream, "http://192.168.0.7:17494/w/k3x9q2/1.0/");
        assert_eq!(r.token.len(), 32);
        assert_eq!(State::load(t.path()), s);
    }

    #[test]
    fn the_invite_code_stays_with_the_war_and_goes_into_the_route() {
        let t = tempfile::tempdir().unwrap();
        let mut s = State::default();
        s.upsert(WarRef { key: Some("K7M2P-9XQ4T".into()), ..lan_war() });
        // Picked again from a list that holds no code: the code stays.
        s.upsert(WarRef { name: "Renamed".into(), ..lan_war() });
        assert_eq!(s.war("k3x9q2").unwrap().key.as_deref(), Some("K7M2P-9XQ4T"));
        s.save(t.path()).unwrap();
        let s = select(t.path(), Some("k3x9q2")).unwrap();
        assert_eq!(s.route().unwrap().key.as_deref(), Some("K7M2P-9XQ4T"));
        let plain = || WarRef { base: Some("http://192.168.0.7:17494".into()), ..lan_war() };
        assert!(valid_pick(&WarRef { key: Some("K7M2P-9XQ4T".into()), ..plain() }));
        for bad in ["", "a b", "x\r\ny: z", "k/../", &"A".repeat(33)] {
            assert!(!valid_pick(&WarRef { key: Some(bad.to_string()), ..plain() }), "{bad:?}");
        }
    }

    #[test]
    fn tokens_are_fresh_hex_of_the_asked_length() {
        let (a, b) = (random_token(16), random_token(16));
        assert_eq!((a.len(), b.len()), (32, 32));
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        assert_eq!(random_token(0), "");
    }

    #[test]
    fn a_state_file_that_no_longer_reads_is_kept_aside() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir(t.path())).unwrap();
        std::fs::write(dir(t.path()).join(STATE), b"{\"wars\": [").unwrap();
        assert_eq!(State::load(t.path()), State::default());
        assert_eq!(std::fs::read(dir(t.path()).join("state.json.bad")).unwrap(), b"{\"wars\": [");
    }

    #[test]
    fn a_local_war_follows_the_server_port_and_needs_one() {
        let mut s = State { server_port: None, ..Default::default() };
        let w = WarRef { host: WarHost::Local, base: None, ..lan_war() };
        assert_eq!(s.war_api(&w), None);
        s.server_port = Some(40100);
        assert_eq!(s.war_api(&w).unwrap(), "http://127.0.0.1:40100/w/k3x9q2/1.0/");
    }

    #[test]
    fn the_war_line_needs_a_sandbox_the_war_accepts() {
        let t = tempfile::tempdir().unwrap();
        let mut s = State::default();
        s.upsert(lan_war());
        s.save(t.path()).unwrap();
        select(t.path(), Some("k3x9q2")).unwrap();
        assert_eq!(war_line(t.path(), None), None, "no Sandbox: no war mode");
        assert!(war_line(t.path(), Some("other")).is_some(), "any Sandbox plays a war by default");
        set_save(t.path(), "k3x9q2", "sb1").unwrap();
        assert!(war_line(t.path(), Some("other")).is_some(), "even once the war has a save of its own");
        let line = war_line(t.path(), Some("sb1")).unwrap();
        let s = State::load(t.path());
        assert_eq!(line.url, format!("http://127.0.0.1:{}/{}/1.0/", s.relay_port.unwrap(), s.token.unwrap()));
        assert_eq!(line.pool, war_pool_tag("k3x9q2"));
        assert_eq!(line.pool.len(), 12);
        assert!(!line.achievements, "achievements stay blocked unless the war allows them");
        assert_eq!(playing_now(t.path(), Some("sb1")).as_deref(), Some("k3x9q2"));
        assert_eq!(playing_now(t.path(), None), None, "an Official save plays the official war");

        // A war that keeps to its own save, and allows achievements, as the LAN host announced.
        let mut s = State::load(t.path());
        s.upsert(WarRef { own_save_only: true, allow_achievements: true, ..lan_war() });
        s.save(t.path()).unwrap();
        assert_eq!(war_line(t.path(), Some("other")), None, "another Sandbox: no war mode");
        assert_eq!(playing_now(t.path(), Some("other")), None);
        assert!(war_line(t.path(), Some("sb1")).unwrap().achievements);
        assert_eq!(war_line(t.path(), Some("sb1")).unwrap().model, None, "a Vanilla war");
        let mut s = State::load(t.path());
        s.upsert(WarRef { model: Some("enhanced".into()), ..lan_war() });
        s.save(t.path()).unwrap();
        assert_eq!(war_line(t.path(), Some("sb1")).unwrap().model.as_deref(), Some("enhanced"));
        select(t.path(), None).unwrap();
        assert_eq!(war_line(t.path(), Some("sb1")), None, "the Official war again");
    }

    #[test]
    fn a_local_war_reads_its_settings_from_the_war_file() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path();
        let local = WarRef { id: "aaa".into(), name: "Home".into(), host: WarHost::Local, save: Some("sb1".into()), own_save_only: true, ..Default::default() };
        assert_eq!(war_flags(data, &local), WarFlags::default(), "no file: the defaults, whatever the entry says");
        let dir = server::server_data(data).join("wars/aaa");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("war.json"), r#"{"id":"aaa","name":"Home","settings":{"points_max":5,"own_save_only":true,"allow_achievements":true}}"#).unwrap();
        assert_eq!(war_flags(data, &local), WarFlags { own_save_only: true, allow_achievements: true });
        std::fs::write(dir.join("war.json"), r#"{"id":"aaa","name":"Home","settings":{"points_max":5}}"#).unwrap();
        assert_eq!(war_flags(data, &local), WarFlags::default(), "an older war file has neither");
        std::fs::write(dir.join("war.json"), r#"{"id":"zzz","name":"Home","settings":{"allow_achievements":true}}"#).unwrap();
        assert_eq!(war_flags(data, &local), WarFlags::default(), "a file of another war counts for nothing");
        assert!(save_allowed(&local, WarFlags { own_save_only: true, ..Default::default() }, "sb1"));
        assert!(!save_allowed(&local, WarFlags { own_save_only: true, ..Default::default() }, "sb2"));
        assert!(save_allowed(&local, WarFlags::default(), "sb2"));
        assert!(!save_allowed(&WarRef { save: None, ..local }, WarFlags { own_save_only: true, ..Default::default() }, "sb2"));
    }

    #[test]
    fn a_war_plays_on_any_sandbox_unless_it_keeps_to_its_own() {
        use crate::saves::SaveKind;
        let w = crate::saves::test_support::world();
        let (data, saves) = (&w.data, w.saves());
        let official = saves.list().unwrap().saves.iter().find(|r| r.kind == SaveKind::Official).unwrap().id.clone();
        let mine = saves.create_sandbox("Mine").unwrap();
        let war = || WarRef { id: "k3x9q2".into(), name: "Night War".into(), host: WarHost::Lan, base: Some("http://192.168.0.7:17494".into()), ..Default::default() };
        let side = crate::settings::game_side_dir(&w.game.saves_dir);
        let line = || crate::settings::parse(&std::fs::read_to_string(side.join(crate::settings::FILE)).unwrap()).war;

        // Picked on a Sandbox of the player's: played there, and no war save is made.
        assert_eq!(play_war(data, &saves, war(), Some(&mine)).unwrap(), mine);
        assert_eq!(State::load(data).war("k3x9q2").unwrap().save, None);
        assert_eq!(line().unwrap().id, "k3x9q2");
        assert_eq!(saves.list().unwrap().saves.len(), 2);

        // No save picked: the war gets its own on first use, and keeps it.
        let own = play_war(data, &saves, war(), None).unwrap();
        assert_ne!(own, mine);
        assert_eq!(State::load(data).war("k3x9q2").unwrap().save.as_deref(), Some(own.as_str()));
        assert_eq!(play_war(data, &saves, war(), None).unwrap(), own);

        // An Official save never plays a custom war, and nothing changes.
        assert!(matches!(play_war(data, &saves, war(), Some(&official)), Err(ObsidianError::WarSave(_))));
        assert!(matches!(play_war(data, &saves, war(), Some("nope")), Err(ObsidianError::WarSave(_))));
        assert_eq!(saves.list().unwrap().active.as_deref(), Some(own.as_str()));

        // A war that keeps to its own save refuses another Sandbox.
        let strict = || WarRef { own_save_only: true, ..war() };
        assert!(matches!(play_war(data, &saves, strict(), Some(&mine)), Err(ObsidianError::WarSave(_))));
        assert_eq!(play_war(data, &saves, strict(), Some(&own)).unwrap(), own);
        // Switched to another Sandbox in Saves, the war line goes: the official war plays.
        saves.activate(&mine).unwrap();
        assert_eq!(line(), None);
        assert_eq!(playing_now(data, Some(&mine)), None);
    }

    #[test]
    fn only_the_enhanced_model_is_kept() {
        assert_eq!(known_model(Some("enhanced".into())).as_deref(), Some("enhanced"));
        assert_eq!(known_model(Some("evolved".into())), None, "an archived Evolved war plays as Vanilla");
        assert_eq!(known_model(Some("vanilla".into())), None);
        assert_eq!(known_model(None), None);
    }

    #[test]
    fn upsert_keeps_the_save_and_unknown_wars_are_refused() {
        let t = tempfile::tempdir().unwrap();
        let mut s = State::default();
        s.upsert(WarRef { save: Some("sb1".into()), ..lan_war() });
        s.upsert(WarRef { name: "Renamed".into(), ..lan_war() });
        assert_eq!(s.wars.len(), 1);
        assert_eq!(s.wars[0].save.as_deref(), Some("sb1"));
        assert_eq!(s.wars[0].name, "Renamed");
        s.save(t.path()).unwrap();
        assert!(matches!(select(t.path(), Some("nope")), Err(ObsidianError::WarUnknown(_))));
        let s = forget(t.path(), "k3x9q2").unwrap();
        assert!(s.wars.is_empty());
    }

    #[test]
    fn picks_are_checked_and_save_names_fit_saves() {
        let w = |host, base: Option<&str>, id: &str| WarRef { id: id.into(), name: "War".into(), host, base: base.map(str::to_string), save: None, computer: None, model: None, key: None, ..Default::default() };
        assert!(valid_pick(&w(WarHost::Local, None, "k3x9q2")));
        assert!(valid_pick(&w(WarHost::Lan, Some("http://192.168.0.7:17494"), "k3x9q2")));
        assert!(!valid_pick(&w(WarHost::Lan, None, "k3x9q2")), "a LAN war needs an address");
        assert!(!valid_pick(&w(WarHost::Lan, Some("file:///c:/x"), "k3x9q2")));
        assert!(!valid_pick(&w(WarHost::Lan, Some("http://h:1/w/x"), "k3x9q2")), "the relay adds the path");
        assert!(!valid_pick(&w(WarHost::Local, None, "../etc")));
        assert_eq!(save_name("  Night War \n"), "Night War");
        assert_eq!(save_name(""), "Custom War");
        assert_eq!(save_name(&"x".repeat(80)).chars().count(), 40);
    }

    #[test]
    fn war_choices_list_live_hosted_wars_then_joined_ones() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path();
        let wars = server::server_data(data).join("wars");
        let put = |id: &str, body: &str| {
            std::fs::create_dir_all(wars.join(id)).unwrap();
            std::fs::write(wars.join(id).join("war.json"), body).unwrap();
        };
        put("bbb", r#"{"id":"bbb","name":"Second","model":"enhanced","state":"paused","season":1,"settings":{"own_save_only":true}}"#);
        put("aaa", r#"{"id":"aaa","name":"first","model":"evolved","state":"running","evolved":{"x":1}}"#);
        put("ccc", r#"{"id":"ccc","name":"Done","state":"won"}"#);
        put("bad", "not json");
        let mut s = State::default();
        s.upsert(WarRef { save: Some("sb9".into()), model: Some("enhanced".into()), allow_achievements: true, ..lan_war() });
        s.save(data).unwrap();
        assert!(war_choices(data).iter().all(|c| c.host == WarHost::Lan), "no package: no hosted wars");

        s.package = Some(Installed { version: "0.3.0".into(), protocol: PROTOCOL, server: "server.exe".into(), ui: "ui".into() });
        s.save(data).unwrap();
        let pkg = package::package_dir(data);
        std::fs::create_dir_all(pkg.join("ui")).unwrap();
        std::fs::write(pkg.join("server.exe"), b"MZ").unwrap();
        std::fs::write(pkg.join("ui/index.html"), b"<html>").unwrap();
        let ids = |v: &[WarChoice]| v.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        let c = war_choices(data);
        assert_eq!(ids(&c), ["aaa", "bbb", "k3x9q2"], "a won war is left out, names sort without case");
        assert_eq!(c[0].model, None, "an Evolved war file still loads, as a Vanilla war");
        assert_eq!(c[1].model.as_deref(), Some("enhanced"));
        assert!(c[1].own_save_only && !c[1].allow_achievements);
        assert_eq!(c[2].save.as_deref(), Some("sb9"));
        assert!(c[2].allow_achievements && !c[2].own_save_only);
        let mut s = State::load(data);
        s.upsert(WarRef { id: "ccc".into(), name: "Done".into(), host: WarHost::Local, base: None, save: None, computer: None, model: None, key: None, ..Default::default() });
        s.playing = Some("ccc".into());
        s.save(data).unwrap();
        assert_eq!(ids(&war_choices(data)), ["ccc", "aaa", "bbb", "k3x9q2"], "the selected war stays even when won");
    }

    #[test]
    fn tokens_differ_and_region_names_read_tab_lines() {
        assert_ne!(random_token(16), random_token(16));
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("commander-names.txt"), "region_bugs_01\tRegião Polaris\nbad line\n\tx\n").unwrap();
        assert_eq!(region_names(t.path()), vec![("region_bugs_01".to_string(), "Região Polaris".to_string())]);
    }
}
