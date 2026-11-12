//! The package server as a child process. It exits when stdin closes, so it never outlives
//! the app or the CLI that started it.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use super::{Installed, State, dir, package, random_token};
use crate::{ObsidianError, Result};

const READY: &str = "COMMANDER READY port=";
const START_WAIT: Duration = Duration::from_secs(15);

pub struct Server {
    child: Child,
    pub port: u16,
    pub token: String,
}

impl Server {
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
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

/// Where the server keeps wars.
pub fn server_data(data: &Path) -> PathBuf {
    dir(data).join("server")
}

/// Starts the installed server on `port` (0: any) and waits for the ready line.
pub fn spawn(data: &Path, installed: &Installed, port: u16, token: &str) -> Result<Server> {
    let pkg = package::package_dir(data);
    let mut cmd = crate::host::command(pkg.join(&installed.server));
    cmd.arg("--data")
        .arg(server_data(data))
        .arg("--port")
        .arg(port.to_string())
        .arg("--token")
        .arg(token)
        .arg("--ui")
        .arg(pkg.join(&installed.ui))
        .current_dir(&pkg)
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
        for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
            if !sent && let Some(rest) = line.strip_prefix(READY) {
                let _ = tx.send(rest.split_whitespace().next().and_then(|p| p.parse::<u16>().ok()));
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
            Err(ObsidianError::CommanderServer(format!("no ready line on port {port}")))
        }
    }
}

/// Starts the server on the port local wars were made on, or any port when that one is taken, and
/// remembers the port: local war addresses follow it.
pub fn start(data: &Path) -> Result<Server> {
    let installed = package::installed(data).ok_or(ObsidianError::CommanderMissing)?;
    let mut state = State::load(data);
    let token = random_token(16);
    let wanted = state.server_port.unwrap_or(0);
    let server = match spawn(data, &installed, wanted, &token) {
        Ok(s) => s,
        Err(_) if wanted != 0 => spawn(data, &installed, 0, &token)?,
        Err(e) => return Err(e),
    };
    if state.server_port != Some(server.port) {
        state.server_port = Some(server.port);
        state.save(data)?;
    }
    Ok(server)
}
