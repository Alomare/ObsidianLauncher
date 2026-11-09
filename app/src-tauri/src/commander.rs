//! The app's side of Commander (custom Galactic Wars, `obsidian_core::commander`): the relay, which
//! runs as long as the app does once a war has been played, and the package's server, a child
//! process that ends with the app (it exits when its stdin closes).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use obsidian_core::commander::{self, Installed, State, WarHost, WarRef, lan, package, relay::Relay};
use obsidian_core::saves::SaveKind;
use obsidian_core::ObsidianError;
use serde::{Deserialize, Serialize};
use tauri::State as TauriState;

use crate::app_settings::AppSettings;
use crate::commands::{CmdError, CmdResult, Ctx, fresh, saves_of};
use crate::snapshot::{Snapshot, find};

const READY: &str = "COMMANDER READY port=";
const START_WAIT: Duration = Duration::from_secs(15);

pub struct Server {
    child: Child,
    pub port: u16,
    pub token: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        // Closing stdin asks the server to stop; a server that doesn't is ended.
        drop(self.child.stdin.take());
        for _ in 0..20 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
pub struct Runtime {
    pub relay: Option<Relay>,
    pub server: Option<Server>,
}

fn rt(ctx: &Ctx) -> std::sync::MutexGuard<'_, Runtime> {
    ctx.commander.lock().unwrap_or_else(|p| p.into_inner())
}

fn server_data(data: &Path) -> PathBuf {
    commander::dir(data).join("server")
}

/// Starts the package's server on `port` (0: any) and waits for its ready line.
fn spawn(data: &Path, installed: &Installed, port: u16, token: &str) -> CmdResult<Server> {
    let dir = package::package_dir(data);
    let mut cmd = Command::new(dir.join(&installed.server));
    cmd.arg("--data")
        .arg(server_data(data))
        .arg("--port")
        .arg(port.to_string())
        .arg("--token")
        .arg(token)
        .arg("--ui")
        .arg(dir.join(&installed.ui))
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window for the server.
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| ObsidianError::CommanderServer(e.to_string()))?;
    let stdout = child.stdout.take().ok_or_else(|| ObsidianError::CommanderServer("no output".into()))?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut sent = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if !sent && let Some(rest) = line.strip_prefix(READY) {
                let port = rest.split_whitespace().next().and_then(|p| p.parse::<u16>().ok());
                let _ = tx.send(port);
                sent = true;
            }
            // Keeps reading, so a chatty server never blocks on a full pipe.
        }
    });
    match rx.recv_timeout(START_WAIT) {
        Ok(Some(port)) => Ok(Server { child, port, token: token.to_string() }),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            Err(ObsidianError::CommanderServer(format!("no ready line on port {port}")).into())
        }
    }
}

/// The server, started if it isn't running. Keeps the port local wars were made on when it can.
fn ensure_server(ctx: &Ctx, runtime: &mut Runtime) -> CmdResult<(u16, String)> {
    if let Some(s) = runtime.server.as_mut() {
        if matches!(s.child.try_wait(), Ok(None)) {
            return Ok((s.port, s.token.clone()));
        }
        runtime.server = None;
    }
    let installed = package::installed(&ctx.data).ok_or(ObsidianError::CommanderMissing)?;
    let mut state = State::load(&ctx.data);
    let token = commander::random_token(16);
    let wanted = state.server_port.unwrap_or(0);
    let server = match spawn(&ctx.data, &installed, wanted, &token) {
        Ok(s) => s,
        // The old port is taken: any port, and local wars follow it.
        Err(_) if wanted != 0 => spawn(&ctx.data, &installed, 0, &token)?,
        Err(e) => return Err(e),
    };
    if state.server_port != Some(server.port) {
        state.server_port = Some(server.port);
        state.save(&ctx.data)?;
    }
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
        && !matches!(s.child.try_wait(), Ok(None))
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

/// A war to play, as the interface or the LAN list names it.
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

fn check_pick(p: &WarPick) -> CmdResult<()> {
    let id_ok = !p.id.is_empty() && p.id.len() <= 32 && p.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    let base_ok = match (&p.host, &p.base) {
        (WarHost::Local, _) => true,
        // `http://<address>:<port>` and nothing more: the relay adds the war's path.
        (WarHost::Lan, Some(b)) => b.len() < 120 && b.strip_prefix("http://").is_some_and(|rest| !rest.is_empty() && !rest.contains(['/', '?', '#', '@', ' '])),
        (WarHost::Lan, None) => false,
    };
    if !id_ok || !base_ok {
        return Err(CmdError { code: "WarUnknown".into(), message: format!("not a war: {}", p.id) });
    }
    Ok(())
}

/// A save name from a war name: Saves takes 1 to 40 characters.
fn save_name(war: &str) -> String {
    let clean: String = war.chars().filter(|c| !c.is_control()).collect();
    let clean = clean.trim();
    let base = if clean.is_empty() { "Custom War" } else { clean };
    base.chars().take(40).collect()
}

/// Selects a war for play: remembers it, gives it a Sandbox save on first use, selects that save
/// (which writes the settings file with the war line) and points the relay at it.
fn select_war(ctx: &Ctx, runtime: &mut Runtime, pick: &WarPick) -> CmdResult<()> {
    check_pick(pick)?;
    let mut state = State::load(&ctx.data);
    state.upsert(WarRef {
        id: pick.id.clone(),
        name: pick.name.chars().take(60).collect(),
        host: pick.host,
        base: pick.base.clone(),
        save: None,
        computer: pick.computer.clone(),
    });
    state.save(&ctx.data)?;
    if pick.host == WarHost::Local {
        ensure_server(ctx, runtime)?;
    }
    let saves = saves_of(ctx)?;
    let known: Vec<String> = saves.list()?.saves.into_iter().filter(|r| r.kind == SaveKind::Sandbox).map(|r| r.id).collect();
    let save = match State::load(&ctx.data).war(&pick.id).and_then(|w| w.save.clone()).filter(|s| known.contains(s)) {
        Some(s) => s,
        None => {
            let s = saves.create_sandbox(&save_name(&pick.name))?;
            commander::set_save(&ctx.data, &pick.id, &s)?;
            s
        }
    };
    commander::select(&ctx.data, Some(&pick.id))?;
    saves.activate(&save)?;
    ensure_relay(ctx, runtime)
}

#[tauri::command]
pub async fn commander_select(ctx: TauriState<'_, Ctx>, war: WarPick) -> CmdResult<CommanderDone> {
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    select_war(&ctx, &mut runtime, &war)?;
    done(&ctx, &mut runtime)
}

/// Back to the official war: no war selected, and the Official save that's in the game's files.
#[tauri::command]
pub async fn commander_official(ctx: TauriState<'_, Ctx>) -> CmdResult<CommanderDone> {
    let _turn = ctx.turn();
    let mut runtime = rt(&ctx);
    commander::select(&ctx.data, None)?;
    let saves = saves_of(&ctx)?;
    let list = saves.list()?;
    let official = list.saves.iter().find(|r| r.kind == SaveKind::Official && r.live).or_else(|| list.saves.iter().find(|r| r.kind == SaveKind::Official));
    if let Some(row) = official {
        saves.activate(&row.id)?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(host: WarHost, base: Option<&str>, id: &str) -> WarPick {
        WarPick { id: id.into(), name: "War".into(), host, base: base.map(str::to_string), computer: None }
    }

    #[test]
    fn picks_are_checked() {
        assert!(check_pick(&pick(WarHost::Local, None, "k3x9q2")).is_ok());
        assert!(check_pick(&pick(WarHost::Lan, Some("http://192.168.0.7:17494"), "k3x9q2")).is_ok());
        assert!(check_pick(&pick(WarHost::Lan, None, "k3x9q2")).is_err(), "a LAN war needs its address");
        assert!(check_pick(&pick(WarHost::Lan, Some("file:///c:/x"), "k3x9q2")).is_err());
        assert!(check_pick(&pick(WarHost::Local, None, "../etc")).is_err());
    }

    #[test]
    fn save_names_fit_saves() {
        assert_eq!(save_name("  Night War \n"), "Night War");
        assert_eq!(save_name(""), "Custom War");
        assert_eq!(save_name(&"x".repeat(80)).chars().count(), 40);
    }
}
