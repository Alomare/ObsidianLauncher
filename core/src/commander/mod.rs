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
//! A war is only played on a dedicated Sandbox save, so nothing from it reaches the Official save,
//! and the loader refuses war mode without one.

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
/// The mod an Evolved war plays through: the galaxy map, the missions and the reports
/// (github.com/Alomare/HD1-EvolvedWarEngine).
pub const EVOLVED_MOD: &str = "alomare.evolvedwar";

/// Where a war runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarHost {
    /// On the Commander server on this PC.
    Local,
    /// On another PC of the local network.
    Lan,
}

/// A war this PC plays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        std::fs::read_to_string(dir(data).join(STATE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
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

    /// Adds a war or updates the one with that id, keeping the save.
    pub fn upsert(&mut self, war: WarRef) {
        match self.wars.iter_mut().find(|w| w.id == war.id) {
            Some(w) => {
                let save = w.save.take();
                *w = WarRef { save: war.save.clone().or(save), ..war };
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
        Some(relay::Route { token: self.token.clone()?, upstream: self.war_api(war)? })
    }
}

/// The war line of the settings file, for the war being played: only with a dedicated Sandbox save
/// selected, which the caller checks against the active Sandbox.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WarLine {
    pub id: String,
    /// Where the game sends war requests: the relay, with the launch token.
    pub url: String,
    /// The tag that keeps the players of this war in a separate matchmaking pool.
    pub pool: String,
}

/// The war line, when a war is selected and `sandbox` is the war save.
pub fn war_line(data: &Path, sandbox: Option<&str>) -> Option<WarLine> {
    let s = State::load(data);
    let war = s.war(s.playing.as_deref()?)?;
    if war.save.as_deref() != sandbox || sandbox.is_none() {
        return None;
    }
    Some(WarLine { id: war.id.clone(), url: format!("http://127.0.0.1:{}/{}/1.0/", s.relay_port?, s.token.as_deref()?), pool: war_pool_tag(&war.id) })
}

/// The pool tag of a war: players of one war find each other and nobody else.
pub fn war_pool_tag(id: &str) -> String {
    use sha2::{Digest, Sha256};
    let d = Sha256::digest(format!("obsidian:war:1\n{id}").as_bytes());
    d.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// A fresh random hex string from the system random source.
pub fn random_token(bytes: usize) -> String {
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
    id_ok && base_ok
}

/// Selects a war for the next launch: remembers it, gives it a Sandbox save on first use, and
/// selects that save, which writes the settings file with the war line. A local war needs the
/// server port known, so start the server first.
pub fn play_war(data: &Path, saves: &crate::saves::Saves, war: WarRef) -> Result<()> {
    if !valid_pick(&war) {
        return Err(ObsidianError::WarUnknown(war.id));
    }
    let id = war.id.clone();
    let name = war.name.clone();
    let mut state = State::load(data);
    state.upsert(WarRef { name: war.name.chars().take(60).collect(), save: None, ..war });
    state.save(data)?;
    let known: Vec<String> = saves.list()?.saves.into_iter().filter(|r| r.kind == crate::saves::SaveKind::Sandbox).map(|r| r.id).collect();
    let save = match State::load(data).war(&id).and_then(|w| w.save.clone()).filter(|s| known.contains(s)) {
        Some(s) => s,
        None => {
            let s = saves.create_sandbox(&save_name(&name))?;
            set_save(data, &id, &s)?;
            s
        }
    };
    select(data, Some(&id))?;
    saves.activate(&save)
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
        WarRef { id: "k3x9q2".into(), name: "Night War".into(), host: WarHost::Lan, base: Some("http://192.168.0.7:17494/".into()), save: None, computer: Some("DESK".into()) }
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
    fn a_local_war_follows_the_server_port_and_needs_one() {
        let mut s = State { server_port: None, ..Default::default() };
        let w = WarRef { host: WarHost::Local, base: None, ..lan_war() };
        assert_eq!(s.war_api(&w), None);
        s.server_port = Some(40100);
        assert_eq!(s.war_api(&w).unwrap(), "http://127.0.0.1:40100/w/k3x9q2/1.0/");
    }

    #[test]
    fn the_war_line_needs_the_war_save_selected() {
        let t = tempfile::tempdir().unwrap();
        let mut s = State::default();
        s.upsert(lan_war());
        s.save(t.path()).unwrap();
        select(t.path(), Some("k3x9q2")).unwrap();
        assert_eq!(war_line(t.path(), None), None, "no Sandbox: no war mode");
        set_save(t.path(), "k3x9q2", "sb1").unwrap();
        assert_eq!(war_line(t.path(), Some("other")), None, "another Sandbox: no war mode");
        let line = war_line(t.path(), Some("sb1")).unwrap();
        let s = State::load(t.path());
        assert_eq!(line.url, format!("http://127.0.0.1:{}/{}/1.0/", s.relay_port.unwrap(), s.token.unwrap()));
        assert_eq!(line.pool, war_pool_tag("k3x9q2"));
        assert_eq!(line.pool.len(), 12);
        select(t.path(), None).unwrap();
        assert_eq!(war_line(t.path(), Some("sb1")), None, "the Official war again");
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
        let w = |host, base: Option<&str>, id: &str| WarRef { id: id.into(), name: "War".into(), host, base: base.map(str::to_string), save: None, computer: None };
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
    fn tokens_differ_and_region_names_read_tab_lines() {
        assert_ne!(random_token(16), random_token(16));
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("commander-names.txt"), "region_bugs_01\tRegião Polaris\nbad line\n\tx\n").unwrap();
        assert_eq!(region_names(t.path()), vec![("region_bugs_01".to_string(), "Região Polaris".to_string())]);
    }
}
