//! The LAN ports in a Linux firewall. Windows asks the player itself the first time a program listens
//! on the network; firewalld (Fedora, Bazzite) and ufw (when turned on, often on Ubuntu and Mint)
//! block the ports without asking. Obsidian opens them only when the player says so, with one
//! password prompt (`pkexec`), and keeps a record in `<data>/lan-firewall.json`. Remove All Commander
//! Data closes what the record lists and nothing else.
//!
//! The commands that run as root are fixed text: no value from a file or the network is part of
//! them.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::Result;

/// The record of what Obsidian opened. Kept outside `<data>/commander`, so it outlives a wipe that
/// left the ports open.
const RECORD: &str = "lan-firewall.json";
/// The war listener takes 17494, or the next free port when that one is in use.
pub const WAR_PORTS: (u16, u16) = (17494, 17499);
/// LAN war announcements.
pub const DISCOVERY_PORT: u16 = super::lan::DISCOVERY_PORT;

const PKEXEC: &str = "/usr/bin/pkexec";
const SH: &str = "/bin/sh";
const FIREWALL_CMD: [&str; 2] = ["/usr/bin/firewall-cmd", "/usr/sbin/firewall-cmd"];
const UFW: [&str; 2] = ["/usr/sbin/ufw", "/usr/bin/ufw"];
const UFW_CONF: &str = "/etc/ufw/ufw.conf";
/// What pkexec answers when the password prompt is closed or the password is wrong.
const PKEXEC_REFUSED: [i32; 2] = [126, 127];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Firewalld,
    Ufw,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// The firewall that is on, if any Obsidian knows.
    pub backend: Option<Backend>,
    /// Whether the LAN ports get through: open in the firewall, or no firewall on.
    pub open: bool,
    /// Whether Obsidian opened ports that are still in the record.
    pub recorded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Record {
    backend: Backend,
}

/// How a run as root ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    Done,
    Declined,
    Failed { message: String },
}

fn first_file(paths: &[&str]) -> Option<PathBuf> {
    paths.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// The firewall that is on: firewalld when running, else ufw when turned on.
fn backend() -> Option<Backend> {
    if cfg!(windows) {
        return None;
    }
    if let Some(cmd) = first_file(&FIREWALL_CMD) {
        // `--state` needs no root and exits 0 only while firewalld runs.
        if Command::new(cmd).arg("--state").output().is_ok_and(|o| o.status.success()) {
            return Some(Backend::Firewalld);
        }
    }
    let ufw_on = std::fs::read_to_string(UFW_CONF).is_ok_and(|t| ufw_enabled(&t));
    (ufw_on && first_file(&UFW).is_some()).then_some(Backend::Ufw)
}

/// Whether `ufw.conf` says the firewall is on.
fn ufw_enabled(conf: &str) -> bool {
    conf.lines().map(str::trim).any(|l| l.eq_ignore_ascii_case("ENABLED=yes"))
}

/// Whether the `firewall-cmd --list-ports` text lets the war ports and the discovery port through.
fn ports_cover(list: &str) -> bool {
    let entries: Vec<(u16, u16, &str)> = list
        .split_whitespace()
        .filter_map(|e| {
            let (range, proto) = e.split_once('/')?;
            let (a, b) = range.split_once('-').unwrap_or((range, range));
            Some((a.parse().ok()?, b.parse().ok()?, proto))
        })
        .collect();
    let covered = |lo: u16, hi: u16, proto: &str| entries.iter().any(|&(a, b, p)| p == proto && a <= lo && hi <= b);
    covered(WAR_PORTS.0, WAR_PORTS.1, "tcp") && covered(DISCOVERY_PORT, DISCOVERY_PORT, "udp")
}

fn load(data: &Path) -> Option<Record> {
    std::fs::read_to_string(data.join(RECORD)).ok().and_then(|t| serde_json::from_str(&t).ok())
}

pub fn status(data: &Path) -> Status {
    let recorded = load(data).is_some();
    let backend = backend();
    let open = match backend {
        None => true,
        // The default zone of some systems (Fedora Workstation) already allows the high ports.
        Some(Backend::Firewalld) => first_file(&FIREWALL_CMD)
            .and_then(|cmd| Command::new(cmd).arg("--list-ports").output().ok())
            .is_some_and(|o| ports_cover(&String::from_utf8_lossy(&o.stdout))),
        // ufw shows rules to root only; the record stands in.
        Some(Backend::Ufw) => recorded,
    };
    Status { backend, open, recorded }
}

/// The port arguments, as each tool spells them.
fn ports(backend: Backend) -> [String; 2] {
    match backend {
        Backend::Firewalld => [format!("{}-{}/tcp", WAR_PORTS.0, WAR_PORTS.1), format!("{DISCOVERY_PORT}/udp")],
        Backend::Ufw => [format!("{}:{}/tcp", WAR_PORTS.0, WAR_PORTS.1), format!("{DISCOVERY_PORT}/udp")],
    }
}

/// The root script that opens (`add`) or closes the ports. Fixed text: the tool by full path, the
/// constant ports.
fn script(backend: Backend, tool: &Path, add: bool) -> String {
    let [tcp, udp] = ports(backend);
    let tool = tool.display();
    match (backend, add) {
        (Backend::Firewalld, true) => format!(
            "{tool} --permanent --add-port={tcp} --add-port={udp} >/dev/null && {tool} --add-port={tcp} --add-port={udp} >/dev/null"
        ),
        (Backend::Firewalld, false) => format!(
            "{tool} --permanent --remove-port={tcp} --remove-port={udp} >/dev/null; {tool} --remove-port={tcp} --remove-port={udp} >/dev/null; exit 0"
        ),
        (Backend::Ufw, true) => format!("{tool} allow {tcp} >/dev/null && {tool} allow {udp} >/dev/null"),
        (Backend::Ufw, false) => format!("{tool} delete allow {tcp} >/dev/null; {tool} delete allow {udp} >/dev/null; exit 0"),
    }
}

fn run_as_root(backend: Backend, add: bool) -> Outcome {
    let tools: &[&str] = match backend {
        Backend::Firewalld => &FIREWALL_CMD,
        Backend::Ufw => &UFW,
    };
    let Some(tool) = first_file(tools) else { return Outcome::Failed { message: "the firewall tool is missing".into() } };
    if !Path::new(PKEXEC).is_file() {
        return Outcome::Failed { message: "pkexec is missing".into() };
    }
    match Command::new(PKEXEC).arg(SH).arg("-c").arg(script(backend, &tool, add)).output() {
        Err(e) => Outcome::Failed { message: e.to_string() },
        Ok(o) if o.status.success() => Outcome::Done,
        Ok(o) if o.status.code().is_some_and(|c| PKEXEC_REFUSED.contains(&c)) => Outcome::Declined,
        Ok(o) => Outcome::Failed { message: String::from_utf8_lossy(&o.stderr).trim().chars().take(300).collect() },
    }
}

/// Opens the LAN ports in the firewall that is on, after the password prompt, and records it.
pub fn open(data: &Path) -> Result<Outcome> {
    let Some(backend) = backend() else { return Ok(Outcome::Done) };
    let outcome = run_as_root(backend, true);
    if outcome == Outcome::Done {
        crate::paths::write_atomic(&data.join(RECORD), serde_json::to_string(&Record { backend })?.as_bytes())?;
    }
    Ok(outcome)
}

/// How many port entries Obsidian opened, by the record: 2 or 0.
pub fn recorded_entries(data: &Path) -> u32 {
    if load(data).is_some() { 2 } else { 0 }
}

/// Closes what the record lists, after the password prompt, and drops the record.
pub fn close(data: &Path) -> Outcome {
    let Some(record) = load(data) else { return Outcome::Done };
    let outcome = run_as_root(record.backend, false);
    if outcome == Outcome::Done {
        let _ = std::fs::remove_file(data.join(RECORD));
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zone_that_allows_the_high_ports_lets_the_wars_through() {
        assert!(ports_cover("1025-65535/tcp 1025-65535/udp"));
        assert!(ports_cover("17494-17499/tcp 17493/udp"));
        assert!(!ports_cover("17494/tcp 17493/udp"), "one war port is not the range");
        assert!(!ports_cover("1025-65535/tcp"), "discovery needs udp");
        assert!(!ports_cover(""));
        assert!(!ports_cover("bad x/y 1-2"));
    }

    #[test]
    fn ufw_counts_as_on_only_when_the_conf_says_so() {
        assert!(ufw_enabled("# comment\nENABLED=yes\nLOGLEVEL=low\n"));
        assert!(!ufw_enabled("ENABLED=no\n"));
        assert!(!ufw_enabled("#ENABLED=yes\n"));
    }

    #[test]
    fn the_root_scripts_are_fixed_text_with_the_constant_ports() {
        let fw = Path::new("/usr/bin/firewall-cmd");
        assert_eq!(
            script(Backend::Firewalld, fw, true),
            "/usr/bin/firewall-cmd --permanent --add-port=17494-17499/tcp --add-port=17493/udp >/dev/null && /usr/bin/firewall-cmd --add-port=17494-17499/tcp --add-port=17493/udp >/dev/null"
        );
        assert!(script(Backend::Firewalld, fw, false).contains("--permanent --remove-port=17494-17499/tcp --remove-port=17493/udp"));
        let ufw = Path::new("/usr/sbin/ufw");
        assert_eq!(script(Backend::Ufw, ufw, true), "/usr/sbin/ufw allow 17494:17499/tcp >/dev/null && /usr/sbin/ufw allow 17493/udp >/dev/null");
        assert!(script(Backend::Ufw, ufw, false).starts_with("/usr/sbin/ufw delete allow 17494:17499/tcp"));
    }

    #[test]
    fn the_record_counts_and_goes_with_a_close_that_has_nothing_to_do() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(recorded_entries(t.path()), 0);
        assert_eq!(close(t.path()), Outcome::Done, "no record, nothing to close");
        std::fs::write(t.path().join(RECORD), r#"{"backend":"ufw"}"#).unwrap();
        assert_eq!(recorded_entries(t.path()), 2);
    }
}
