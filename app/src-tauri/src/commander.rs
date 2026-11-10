//! The app's side of Commander (custom Galactic Wars, `obsidian_core::commander`): the relay, which
//! runs as long as the app does once a war has been played, and the package's server, a child
//! process that ends with the app.

use std::path::PathBuf;
use std::time::Duration;

use obsidian_core::commander::{self, Installed, State, WarHost, WarRef, lan, package, relay::Relay, server::Server};
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

/// The relay, started on its port if it isn't running, pointed at the war being played.
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

/// At app start: a war selected last time gets its relay, and its server when it's hosted here, so
/// a game started from Steam still reaches it.
pub fn resume(ctx: &Ctx) {
    let state = State::load(&ctx.data);
    let Some(war) = state.playing.as_deref().and_then(|id| state.war(id)).cloned() else { return };
    let mut runtime = rt(ctx);
    if war.host == WarHost::Local && package::installed(&ctx.data).is_some() {
        let _ = ensure_server(ctx, &mut runtime);
    }
    let _ = ensure_relay(ctx, &mut runtime);
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
    /// Region and city names in the player's language, from the last war launch.
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
    // A local war being played follows the server's port.
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
}

impl WarPick {
    fn into_ref(self) -> WarRef {
        WarRef { id: self.id, name: self.name, host: self.host, base: self.base, save: None, computer: self.computer }
    }
}

#[tauri::command]
pub async fn commander_select(ctx: TauriState<'_, Ctx>, war: WarPick) -> CmdResult<CommanderDone> {
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    let war = war.into_ref();
    if !commander::valid_pick(&war) {
        return Err(CmdError { code: "WarUnknown".into(), message: format!("not a war: {}", war.id) });
    }
    if war.host == WarHost::Local {
        ensure_server(&ctx, &mut runtime)?;
    }
    commander::play_war(&ctx.data, &saves_of(&ctx)?, war)?;
    ensure_relay(&ctx, &mut runtime)?;
    done(&ctx, &mut runtime)
}

/// Back to the official war: no war selected, and the Official save that's in the game's files.
#[tauri::command]
pub async fn commander_official(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderDone> {
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    commander::play_official(&ctx.data, &saves_of(&ctx)?)?;
    ensure_relay(&ctx, &mut runtime)?;
    done(&ctx, &mut runtime)
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
