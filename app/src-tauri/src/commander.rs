//! The app side of Commander (custom Galactic Wars, `obsidian_core::commander`): the relay, which
//! runs as long as the app does once a war has been played, and the package server, a child
//! process that ends with the app.

use std::path::PathBuf;
use std::time::Duration;

use obsidian_core::commander::{self, Installed, State, WarHost, WarRef, client, lan, package, relay::Relay, server::Server, wipe};
use serde::{Deserialize, Serialize};
use tauri::State as TauriState;

use crate::app_settings::AppSettings;
use crate::commands::{CmdError, CmdResult, Ctx, fresh, saves_of};
use crate::snapshot::{Snapshot, find};

#[derive(Default)]
pub struct Runtime {
    pub relay: Option<Relay>,
    pub server: Option<Server>,
}

fn rt(ctx: &Ctx) -> std::sync::MutexGuard<'_, Runtime> {
    ctx.commander.lock().unwrap_or_else(|p| p.into_inner())
}

/// The server, started if it isn't running.
fn ensure_server(ctx: &Ctx, runtime: &mut Runtime) -> CmdResult<(u16, String)> {
    if let Some(s) = runtime.server.as_mut() {
        if s.alive() {
            return Ok((s.port, s.token.clone()));
        }
        runtime.server = None;
    }
    let server = commander::server::start(&ctx.data)?;
    let out = (server.port, server.token.clone());
    runtime.server = Some(server);
    Ok(out)
}

/// The relay, started on the chosen port if it isn't running, pointed at the war being played.
fn ensure_relay(ctx: &Ctx, runtime: &mut Runtime) -> CmdResult<()> {
    if runtime.relay.is_none() {
        let port = commander::ensure_relay_port(&ctx.data)?;
        runtime.relay = Some(Relay::start(port)?);
    }
    let route = State::load(&ctx.data).route();
    if let Some(r) = &runtime.relay {
        r.set_route(route);
    }
    Ok(())
}

/// At app start: a war selected last time gets the relay, and the server when it's hosted here, so
/// a game started from Steam still reaches it.
pub fn resume(ctx: &Ctx) {
    resume_with(ctx, &mut rt(ctx));
}

fn resume_with(ctx: &Ctx, runtime: &mut Runtime) {
    let state = State::load(&ctx.data);
    let Some(war) = state.playing.as_deref().and_then(|id| state.war(id)).cloned() else { return };
    if war.host == WarHost::Local && package::installed(&ctx.data).is_some() {
        let _ = ensure_server(ctx, runtime);
    }
    let _ = ensure_relay(ctx, runtime);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub port: u16,
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommanderView {
    pub protocol: u32,
    pub installed: Option<Installed>,
    pub wars: Vec<WarRef>,
    pub playing: Option<String>,
    pub server: Option<ServerInfo>,
    /// Region and city names in the player language, from the last war launch.
    pub names: Vec<(String, String)>,
}

fn view(ctx: &Ctx, runtime: &mut Runtime) -> CommanderView {
    if let Some(s) = runtime.server.as_mut()
        && !s.alive()
    {
        runtime.server = None;
    }
    let state = State::load(&ctx.data);
    let names = find(&AppSettings::load(&ctx.data))
        .map(|g| commander::region_names(&obsidian_core::settings::game_side_dir(&g.saves_dir)))
        .unwrap_or_default();
    CommanderView {
        protocol: commander::PROTOCOL,
        installed: package::installed(&ctx.data),
        wars: state.wars,
        playing: state.playing,
        server: runtime.server.as_ref().map(|s| ServerInfo { port: s.port, token: s.token.clone() }),
        names,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommanderDone {
    pub commander: CommanderView,
    pub snapshot: Snapshot,
}

fn done(ctx: &Ctx, runtime: &mut Runtime) -> CmdResult<CommanderDone> {
    Ok(CommanderDone { commander: view(ctx, runtime), snapshot: fresh(ctx)? })
}

#[tauri::command]
pub async fn commander_state(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderView> {
    Ok(view(&ctx, &mut rt(&ctx)))
}

/// Installs a package ZIP (or folder), stopping the server that runs the old one.
#[tauri::command]
pub async fn commander_install(ctx: TauriState<'_, Ctx>, path: PathBuf) -> CmdResult<CommanderView> {
    let mut runtime = rt(&ctx);
    runtime.server = None;
    package::install(&ctx.data, &path)?;
    Ok(view(&ctx, &mut runtime))
}

/// Downloads the newest released package from GitHub and installs it, stopping the server that
/// runs the old one.
#[tauri::command]
pub async fn commander_fetch(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderView> {
    let mut runtime = rt(&ctx);
    runtime.server = None;
    package::fetch(&ctx.data, &obsidian_core::updates::GitHub::default())?;
    Ok(view(&ctx, &mut runtime))
}

/// The firewall state for the COMMANDER tab.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallView {
    /// Linux, where Obsidian offers to open the LAN ports; Windows asks the player by itself.
    pub linux: bool,
    #[serde(flatten)]
    pub status: commander::firewall::Status,
}

/// The Linux firewall and the LAN ports. Reading it runs `firewall-cmd`, so it stays off the window's thread.
#[tauri::command]
pub async fn commander_firewall(ctx: TauriState<'_, Ctx>) -> CmdResult<FirewallView> {
    Ok(FirewallView { linux: cfg!(not(windows)), status: commander::firewall::status(&ctx.data) })
}

/// Opens the LAN ports in the Linux firewall, after the system password prompt.
#[tauri::command]
pub async fn commander_firewall_open(ctx: TauriState<'_, Ctx>) -> CmdResult<commander::firewall::Outcome> {
    Ok(commander::firewall::open(&ctx.data)?)
}

#[tauri::command]
pub async fn commander_uninstall(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderView> {
    let mut runtime = rt(&ctx);
    runtime.server = None;
    package::uninstall(&ctx.data)?;
    Ok(view(&ctx, &mut runtime))
}

#[tauri::command]
pub async fn commander_server_start(ctx: TauriState<'_, Ctx>) -> CmdResult<ServerInfo> {
    let mut runtime = rt(&ctx);
    let (port, token) = ensure_server(&ctx, &mut runtime)?;
    // A local war being played follows the server port.
    ensure_relay(&ctx, &mut runtime)?;
    Ok(ServerInfo { port, token })
}

#[tauri::command]
pub async fn commander_server_stop(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderView> {
    let mut runtime = rt(&ctx);
    runtime.server = None;
    Ok(view(&ctx, &mut runtime))
}

/// A war to play, as the war screens or the LAN list name it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarPick {
    pub id: String,
    pub name: String,
    pub host: WarHost,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub computer: Option<String>,
    /// "enhanced" for an Enhanced war, which plays through the Enhanced War Client mod.
    #[serde(default)]
    pub model: Option<String>,
    /// The invite code of a war joined by address.
    #[serde(default)]
    pub key: Option<String>,
    /// The war settings as the LAN list found them. The server is asked again on selecting, and a
    /// war hosted here is read from the war file, so these count only when the host can't be reached.
    #[serde(default)]
    pub own_save_only: bool,
    #[serde(default)]
    pub allow_achievements: bool,
}

impl WarPick {
    fn into_ref(self) -> WarRef {
        WarRef {
            id: self.id,
            name: self.name,
            host: self.host,
            base: self.base,
            save: None,
            computer: self.computer,
            model: commander::known_model(self.model),
            key: self.key.filter(|k| !k.is_empty()),
            own_save_only: self.own_save_only,
            allow_achievements: self.allow_achievements,
        }
    }
}

/// Selects a war for the next launch, on `save` (a Sandbox the war accepts) or, with none, on the
/// Sandbox made for the war.
#[tauri::command]
pub async fn commander_select(ctx: TauriState<'_, Ctx>, war: WarPick, save: Option<String>) -> CmdResult<CommanderDone> {
    let war = war.into_ref();
    if !commander::valid_pick(&war) {
        return Err(CmdError { code: "WarUnknown".into(), message: format!("not a war: {}", war.id) });
    }
    let enhanced = war.model.as_deref() == Some(commander::ENHANCED_MODEL);
    // A LAN war: its settings as the host states them now. Asked outside the turn, as downloads are.
    let war = commander::with_host_flags(war);
    // An Enhanced war plays through the Enhanced War Client mod, downloaded first when the library
    // lacks it.
    let staged = if enhanced { client_download(&ctx)? } else { None };
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    if enhanced {
        client::switch_on(&ctx.data, staged.as_ref())?;
    }
    if war.host == WarHost::Local {
        ensure_server(&ctx, &mut runtime)?;
    }
    commander::play_war(&ctx.data, &saves_of(&ctx)?, war, save.as_deref())?;
    ensure_relay(&ctx, &mut runtime)?;
    done(&ctx, &mut runtime)
}

/// The newest Enhanced War Client release when the library lacks the mod. The download runs outside
/// the turn, as mod updates do.
fn client_download(ctx: &Ctx) -> CmdResult<Option<obsidian_core::updates::Staged>> {
    let missing = {
        let _turn = ctx.turn();
        client::missing(&ctx.data)?
    };
    Ok(if missing { Some(client::fetch(&obsidian_core::updates::GitHub::default())?) } else { None })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientDone {
    pub switched: client::Switched,
    pub snapshot: Snapshot,
}

/// After the war screens create an Enhanced war: the Enhanced War Client mod in the library and on
/// in the active profile, so the host plays it like everyone who joins.
#[tauri::command]
pub async fn commander_enhanced_client(ctx: TauriState<'_, Ctx>) -> CmdResult<ClientDone> {
    let staged = client_download(&ctx)?;
    let _turn = ctx.turn();
    let switched = client::switch_on(&ctx.data, staged.as_ref())?;
    Ok(ClientDone { switched, snapshot: fresh(&ctx)? })
}

/// The most a war file brought in with Import may be.
const MAX_WAR_FILE: u64 = 16 * 1024 * 1024;

/// Reads a war file the player picked for Import in the Commander bar, for the war screens to take
/// in: a `.json` file of at most 16 MB, as text. Anything else is refused.
#[tauri::command]
pub async fn commander_read_war_file(path: PathBuf) -> CmdResult<String> {
    Ok(read_war_file(&path)?)
}

fn read_war_file(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::{Error, ErrorKind, Read};
    let json = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if !json {
        return Err(Error::new(ErrorKind::InvalidInput, "a war file is a .json file"));
    }
    let meta = std::fs::metadata(path)?;
    if !meta.is_file() || meta.len() > MAX_WAR_FILE {
        return Err(Error::new(ErrorKind::InvalidInput, "not a war file of 16 MB or less"));
    }
    let mut text = String::new();
    std::fs::File::open(path)?.take(MAX_WAR_FILE + 1).read_to_string(&mut text)?;
    if text.len() as u64 > MAX_WAR_FILE {
        return Err(Error::new(ErrorKind::InvalidInput, "not a war file of 16 MB or less"));
    }
    Ok(text)
}

/// Back to the official war: no war selected, and the Official save that's in the game files.
#[tauri::command]
pub async fn commander_official(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderDone> {
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    commander::play_official(&ctx.data, &saves_of(&ctx)?)?;
    ensure_relay(&ctx, &mut runtime)?;
    done(&ctx, &mut runtime)
}

/// The official war on a save the player picked in the Play choice.
#[tauri::command]
pub async fn commander_vanilla(ctx: TauriState<'_, Ctx>, save: String) -> CmdResult<CommanderDone> {
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    commander::play_vanilla(&ctx.data, &saves_of(&ctx)?, &save)?;
    ensure_relay(&ctx, &mut runtime)?;
    done(&ctx, &mut runtime)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    pub playing: Option<String>,
    pub wars: Vec<commander::WarChoice>,
}

/// The wars the Play button can start, and the one it starts now: the war selected, when the save
/// the game boots is a Sandbox that war accepts. Otherwise the official war plays.
#[tauri::command]
pub async fn commander_choices(ctx: TauriState<'_, Ctx>) -> CmdResult<Choices> {
    let playing = commander::playing_now(&ctx.data, obsidian_core::saves::active_sandbox(&ctx.data).as_deref());
    Ok(Choices { playing, wars: commander::war_choices(&ctx.data) })
}

/// The version of a newer Commander package on GitHub, if there is one.
#[tauri::command]
pub async fn commander_package_check(ctx: TauriState<'_, Ctx>) -> CmdResult<Option<String>> {
    Ok(package::newer(&ctx.data, &obsidian_core::updates::GitHub::default())?)
}

#[tauri::command]
pub async fn commander_forget(ctx: TauriState<'_, Ctx>, id: String) -> CmdResult<CommanderView> {
    let mut runtime = rt(&ctx);
    let state = commander::forget(&ctx.data, &id)?;
    if state.playing.is_none()
        && let Some(r) = &runtime.relay
    {
        r.set_route(None);
    }
    Ok(view(&ctx, &mut runtime))
}

/// Listens for wars announced on the local network for `ms` milliseconds (at most 10 s).
#[tauri::command]
pub async fn commander_scan(ms: u64) -> CmdResult<Vec<lan::Found>> {
    Ok(lan::scan(Duration::from_millis(ms.clamp(500, 10_000)))?)
}

/// The wars open at an address the player typed (`host` or `host:port`), with the invite code the
/// host gave: the way into a war over the internet.
#[tauri::command]
pub async fn commander_ask(address: String, key: String) -> CmdResult<Vec<lan::Found>> {
    Ok(lan::ask(&address, &key)?)
}

/// The saves and the game-side folder, when the game and a Steam account are found.
fn saves_and_side(ctx: &Ctx) -> (Option<obsidian_core::saves::Saves>, Option<PathBuf>) {
    let side = find(&AppSettings::load(&ctx.data)).map(|g| obsidian_core::settings::game_side_dir(&g.saves_dir));
    (saves_of(ctx).ok(), side)
}

/// What Commander has on this PC, for the remove dialog.
#[tauri::command]
pub async fn commander_footprint(ctx: TauriState<'_, Ctx>) -> CmdResult<wipe::Footprint> {
    let _turn = ctx.turn();
    let (saves, side) = saves_and_side(&ctx);
    Ok(wipe::footprint(&ctx.data, saves.as_ref(), side.as_deref()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommanderWipeDone {
    pub commander: CommanderView,
    pub snapshot: Snapshot,
    pub report: wipe::WipeReport,
}

/// Removes everything Commander left on this PC; `saves` deletes the war Sandbox saves too, and
/// `firewall` the server firewall rules (Windows asks for admin). The server and the relay stop
/// first. Refused while the game runs.
#[tauri::command]
pub async fn commander_wipe(ctx: TauriState<'_, Ctx>, saves: bool, firewall: bool) -> CmdResult<CommanderWipeDone> {
    let _turn = ctx.turn();
    obsidian_core::game::ensure_closed()?;
    let mut runtime = rt(&ctx);
    // The server process ends and the relay port closes before the files go.
    runtime.server = None;
    runtime.relay = None;
    let (s, side) = saves_and_side(&ctx);
    let report = match wipe::wipe(&ctx.data, s.as_ref(), side.as_deref(), saves, firewall) {
        Ok(r) => r,
        Err(e) => {
            // A war still selected gets the relay and server back.
            resume_with(&ctx, &mut runtime);
            return Err(e.into());
        }
    };
    Ok(CommanderWipeDone { commander: view(&ctx, &mut runtime), snapshot: fresh(&ctx)?, report })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_json_file_of_16_mb_or_less_is_read() {
        let t = tempfile::tempdir().unwrap();
        let ok = t.path().join("war.JSON");
        std::fs::write(&ok, br#"{"id":"k3x9q2"}"#).unwrap();
        assert_eq!(read_war_file(&ok).unwrap(), r#"{"id":"k3x9q2"}"#);
        let txt = t.path().join("war.txt");
        std::fs::write(&txt, b"{}").unwrap();
        assert!(read_war_file(&txt).is_err());
        assert!(read_war_file(t.path()).is_err(), "a folder");
        assert!(read_war_file(&t.path().join("none.json")).is_err());
        let big = t.path().join("big.json");
        std::fs::File::create(&big).unwrap().set_len(MAX_WAR_FILE + 1).unwrap();
        assert!(read_war_file(&big).is_err());
        let bin = t.path().join("bin.json");
        std::fs::write(&bin, [0xffu8, 0xfe, 0x00]).unwrap();
        assert!(read_war_file(&bin).is_err(), "not text");
    }
}
