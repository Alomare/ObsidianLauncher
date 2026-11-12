//! Removing everything Commander left on this PC: the package, the wars hosted here, the war list,
//! the names file the loader wrote and, when asked, the war Sandbox saves and the firewall rules
//! Windows made for the server. The app stops the server and the relay before a wipe.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{NAMES_FILE, State, WarHost, dir, package, play_official, select, server};
use crate::saves::{SaveKind, Saves};
use crate::{ObsidianError, Result};

/// The server file name when the war list doesn't name it.
const DEFAULT_SERVER: &str = "commander-server.exe";
const REMOVE_TRIES: u32 = 8;
/// What Windows answers when the admin prompt is declined (ERROR_CANCELLED).
const DECLINED: i32 = 1223;

/// A war Sandbox save.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRef {
    pub id: String,
    pub name: String,
}

/// What Commander has on this PC.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Footprint {
    /// The installed package version. Empty when the folder is there and the version isn't known.
    pub package: Option<String>,
    /// Wars hosted on this PC, in the server folder.
    pub hosted: u32,
    /// Wars of other PCs on the local network that this PC joined.
    pub joined: u32,
    /// The war Sandbox saves that are still in Saves.
    pub saves: Vec<SaveRef>,
    /// Whether the region names the loader wrote are in the game-side folder.
    pub names_file: bool,
    /// Inbound firewall rules for the server and for Obsidian, which Windows adds when the server
    /// opens a war to the network and when Obsidian looks for wars on it.
    pub firewall_rules: u32,
    /// The size of `<data>/commander`.
    pub bytes: u64,
}

/// What became of the firewall rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FirewallOutcome {
    /// The wipe left the firewall alone.
    NotAsked,
    /// There were no rules for the server.
    None,
    Removed { count: u32 },
    /// The admin prompt was declined, so the rules stay.
    Declined,
    /// The rules stay, for this reason.
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WipeReport {
    pub removed_saves: u32,
    /// The war saves that couldn't be deleted.
    pub kept_saves: Vec<String>,
    pub firewall: FirewallOutcome,
}

/// How a run as admin ended.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Elevated {
    Done,
    Declined,
    Failed(String),
}

/// What a wipe asks of the system besides files. Tests fake it.
trait Machine {
    /// The output of `netsh advfirewall firewall show rule name=all verbose`, as netsh wrote it.
    /// None when it can't be read.
    fn firewall_rules(&self) -> Option<Vec<u8>>;
    /// Deletes the firewall rules for each of `programs`, as admin, with one prompt.
    fn delete_firewall_rules(&self, programs: &[PathBuf]) -> Elevated;
    /// The program file of this Obsidian, as the firewall names it.
    fn own_exe(&self) -> Option<PathBuf>;
    /// Whether a process with this file name runs.
    fn running(&self, exe: &str) -> bool;
    /// The LAN port entries Obsidian opened in a Linux firewall, by the record in `data`.
    fn opened_ports(&self, _data: &Path) -> u32 {
        0
    }
    /// Closes them, with the password prompt.
    fn close_ports(&self, _data: &Path) -> Elevated {
        Elevated::Done
    }
}

struct Live;

/// A Windows tool by full path, so a file of the same name beside the app or in the working
/// folder is never the one that runs, least of all as admin.
#[cfg_attr(not(windows), allow(dead_code))]
fn system_tool(relative: &str) -> PathBuf {
    let root = std::env::var_os("SystemRoot").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    root.join("System32").join(relative)
}

#[cfg(windows)]
fn hidden(program: &Path) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new(program);
    // No console window.
    cmd.creation_flags(0x0800_0000);
    cmd
}

impl Machine for Live {
    #[cfg(windows)]
    fn firewall_rules(&self) -> Option<Vec<u8>> {
        // netsh exits with 1 when no rule matches; the text says so and is read either way.
        hidden(&system_tool("netsh.exe")).args(["advfirewall", "firewall", "show", "rule", "name=all", "verbose"]).output().ok().map(|o| o.stdout)
    }

    #[cfg(not(windows))]
    fn firewall_rules(&self) -> Option<Vec<u8>> {
        Some(Vec::new())
    }

    #[cfg(windows)]
    fn delete_firewall_rules(&self, programs: &[PathBuf]) -> Elevated {
        let script: Vec<u8> = elevated_delete_script(programs).encode_utf16().flat_map(u16::to_le_bytes).collect();
        let out = hidden(&system_tool(r"WindowsPowerShell\v1.0\powershell.exe")).args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &base64(&script)]).output();
        match out {
            Err(e) => Elevated::Failed(e.to_string()),
            Ok(o) => match o.status.code() {
                Some(0) => Elevated::Done,
                Some(DECLINED) => Elevated::Declined,
                code => {
                    let why = String::from_utf8_lossy(&o.stderr).trim().to_string();
                    Elevated::Failed(if why.is_empty() { format!("netsh ended with code {}", code.unwrap_or(-1)) } else { why })
                }
            },
        }
    }

    #[cfg(not(windows))]
    fn delete_firewall_rules(&self, _programs: &[PathBuf]) -> Elevated {
        Elevated::Failed("firewall rules are only removed on Windows".into())
    }

    fn own_exe(&self) -> Option<PathBuf> {
        std::env::current_exe().ok()
    }

    #[cfg(not(windows))]
    fn opened_ports(&self, data: &Path) -> u32 {
        super::firewall::recorded_entries(data)
    }

    #[cfg(not(windows))]
    fn close_ports(&self, data: &Path) -> Elevated {
        match super::firewall::close(data) {
            super::firewall::Outcome::Done => Elevated::Done,
            super::firewall::Outcome::Declined => Elevated::Declined,
            super::firewall::Outcome::Failed { message } => Elevated::Failed(message),
        }
    }

    fn running(&self, exe: &str) -> bool {
        crate::game::is_running_named(exe)
    }
}

/// A PowerShell single-quoted string: it ends at any kind of single quote; doubled, each stays.
#[cfg_attr(not(windows), allow(dead_code))]
fn ps_quoted(s: &str) -> String {
    let mut quoted = String::with_capacity(s.len() + 2);
    quoted.push('\'');
    for c in s.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}') {
            quoted.push(c);
        }
        quoted.push(c);
    }
    quoted.push('\'');
    quoted
}

/// What runs as admin: netsh once for each program, by full path. It exits 0 when netsh deleted
/// rules for any of them. The paths are PowerShell strings inside the script, never part of a
/// command line.
#[cfg_attr(not(windows), allow(dead_code))]
fn admin_delete_script(programs: &[PathBuf]) -> String {
    let list: Vec<String> = programs.iter().map(|p| ps_quoted(&p.display().to_string())).collect();
    format!(
        "$code = 1\n\
         foreach ($p in @({})) {{\n\
         & {} advfirewall firewall delete rule name=all \"program=$p\" | Out-Null\n\
         if ($LASTEXITCODE -eq 0) {{ $code = 0 }}\n\
         }}\n\
         exit $code\n",
        list.join(", "),
        ps_quoted(&system_tool("netsh.exe").display().to_string())
    )
}

/// PowerShell that starts `admin_delete_script` as admin, which is when Windows asks. The inner
/// script travels as `-EncodedCommand`, so no path meets a command line parser. It exits with the
/// inner code, or 1223 when the admin prompt is declined.
#[cfg_attr(not(windows), allow(dead_code))]
fn elevated_delete_script(programs: &[PathBuf]) -> String {
    let inner: Vec<u8> = admin_delete_script(programs).encode_utf16().flat_map(u16::to_le_bytes).collect();
    let shell = ps_quoted(&system_tool(r"WindowsPowerShell\v1.0\powershell.exe").display().to_string());
    format!(
        "$i = New-Object System.Diagnostics.ProcessStartInfo\n\
         $i.FileName = {shell}\n\
         $i.Arguments = '-NoProfile -NonInteractive -EncodedCommand {}'\n\
         $i.Verb = 'runas'\n\
         $i.UseShellExecute = $true\n\
         $i.WindowStyle = 'Hidden'\n\
         try {{ $p = [System.Diagnostics.Process]::Start($i); $p.WaitForExit(); exit $p.ExitCode }}\n\
         catch {{ $e = $_.Exception; while ($e.InnerException) {{ $e = $e.InnerException }}; if ($e.NativeErrorCode -eq {DECLINED}) {{ exit {DECLINED} }}; [Console]::Error.WriteLine($e.Message); exit 2 }}\n",
        base64(&inner)
    )
}

/// Standard base64 with padding, for `-EncodedCommand`.
#[cfg_attr(not(windows), allow(dead_code))]
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            out.push(if i <= chunk.len() { ABC[((n >> (18 - 6 * i)) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

/// A path or a line of netsh text, for comparing: trimmed, lower case, `\` separators.
fn plain(s: &str) -> String {
    s.trim().to_ascii_lowercase().replace('/', "\\")
}

/// How many rules in the netsh text are for `program`. netsh writes labels in the system
/// language and text in the console code page, so a rule is found by a line whose value is
/// the path, with every byte past ASCII taken as `?`: one per letter in an OEM code page, or one per
/// UTF-8 byte.
fn count_rules(text: &[u8], program: &Path) -> u32 {
    let path = program.to_string_lossy();
    let (mut oem, mut utf8) = (String::new(), String::new());
    for c in path.chars() {
        if c.is_ascii() {
            oem.push(c);
            utf8.push(c);
        } else {
            oem.push('?');
            utf8.extend(std::iter::repeat_n('?', c.len_utf8()));
        }
    }
    let wanted = [plain(&oem), plain(&utf8)];
    let found = text
        .split(|b| *b == b'\n')
        .map(|line| plain(&line.iter().map(|&b| if b.is_ascii() { b as char } else { '?' }).collect::<String>()))
        .filter(|line| {
            let value = line.split_once(':').map(|(_, v)| v.trim());
            wanted.iter().any(|w| line == w || value == Some(w.as_str()))
        })
        .count();
    found as u32
}

/// The server as the firewall names it.
fn server_exe(data: &Path, state: &State) -> PathBuf {
    let name = state.package.as_ref().map_or(DEFAULT_SERVER, |p| p.server.as_str());
    let exe = package::package_dir(data).join(name);
    std::path::absolute(&exe).unwrap_or(exe)
}

/// The programs whose firewall rules are Commander's: the server, and this Obsidian, which Windows
/// asks about the first time it listens for wars on the network.
fn firewall_programs(m: &dyn Machine, data: &Path, state: &State) -> Vec<PathBuf> {
    let mut all = vec![server_exe(data, state)];
    if let Some(own) = m.own_exe().filter(|own| !all.contains(own)) {
        all.push(own);
    }
    all
}

fn count_all(text: &[u8], programs: &[PathBuf]) -> u32 {
    programs.iter().map(|p| count_rules(text, p)).sum()
}

/// The war saves, once each.
fn war_saves(state: &State) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for id in state.wars.iter().filter_map(|w| w.save.clone()) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// The saves of `ids` that are still Sandbox saves in Saves.
fn existing_saves(saves: &Saves, ids: &[String]) -> Result<Vec<SaveRef>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(saves.list()?.saves.into_iter().filter(|r| r.kind == SaveKind::Sandbox && ids.contains(&r.id)).map(|r| SaveRef { id: r.id, name: r.name }).collect())
}

/// The wars in the server folder: a folder with a `war.json` each.
fn hosted_wars(data: &Path) -> u32 {
    let wars = server::server_data(data).join("wars");
    let Ok(rd) = std::fs::read_dir(wars) else { return 0 };
    rd.flatten().filter(|e| e.path().join("war.json").is_file()).count() as u32
}

/// The size of a file, or of a folder with all inside it. Links aren't followed.
fn size_of(p: &Path) -> u64 {
    let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
    if !m.is_dir() {
        return m.len();
    }
    std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| size_of(&e.path())).sum()
}

/// Removes a folder, trying again for a moment: Windows holds the file of a just-ended program a little
/// while, and an antivirus or the search indexer may hold others.
fn remove_tree(p: &Path) -> Result<()> {
    for _ in 1..REMOVE_TRIES {
        match std::fs::remove_dir_all(p) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(125)),
        }
    }
    match std::fs::remove_dir_all(p) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// What Commander has on this PC. `saves` and `game_side` are None when the game isn't found; those
/// parts then count nothing.
pub fn footprint(data: &Path, saves: Option<&Saves>, game_side: Option<&Path>) -> Footprint {
    footprint_on(&Live, data, saves, game_side)
}

fn footprint_on(m: &dyn Machine, data: &Path, saves: Option<&Saves>, game_side: Option<&Path>) -> Footprint {
    let state = State::load(data);
    Footprint {
        package: package::package_dir(data).is_dir().then(|| state.package.as_ref().map(|p| p.version.clone()).unwrap_or_default()),
        hosted: hosted_wars(data),
        joined: state.wars.iter().filter(|w| w.host == WarHost::Lan).count() as u32,
        saves: saves.and_then(|s| existing_saves(s, &war_saves(&state)).ok()).unwrap_or_default(),
        names_file: game_side.is_some_and(|g| g.join(NAMES_FILE).is_file()),
        firewall_rules: m.firewall_rules().map_or(0, |t| count_all(&t, &firewall_programs(m, data, &state))) + m.opened_ports(data),
        bytes: size_of(&dir(data)),
    }
}

/// Removes everything Commander left on this PC, in this order: back to the Official war (so the
/// settings file loses the war line and the Official save is active again), the war saves when
/// `delete_saves`, the server firewall rules when `firewall` (Windows asks for admin), then
/// `<data>/commander` and the names file. A save that can't be deleted is skipped and listed.
/// Refused while the game runs (checked through `saves`) or while the package server runs; the
/// app stops the server first.
pub fn wipe(data: &Path, saves: Option<&Saves>, game_side: Option<&Path>, delete_saves: bool, firewall: bool) -> Result<WipeReport> {
    wipe_on(&Live, data, saves, game_side, delete_saves, firewall)
}

fn wipe_on(m: &dyn Machine, data: &Path, saves: Option<&Saves>, game_side: Option<&Path>, delete_saves: bool, firewall: bool) -> Result<WipeReport> {
    if let Some(s) = saves {
        s.closed()?;
    }
    let state = State::load(data);
    // Found before the folder goes: the rules name the server path.
    let exe = server_exe(data, &state);
    if package::package_dir(data).is_dir() && exe.file_name().is_some_and(|n| m.running(&n.to_string_lossy())) {
        return Err(ObsidianError::CommanderRunning);
    }
    let war_saves = war_saves(&state);
    match saves {
        Some(s) => {
            // A war save still selected goes back to the Official one too, so it can be deleted.
            let on_war_save = delete_saves && s.active()?.is_some_and(|a| war_saves.contains(&a));
            if state.playing.is_some() || on_war_save {
                play_official(data, s)?;
            }
        }
        None if state.playing.is_some() => {
            select(data, None)?;
        }
        None => {}
    }
    let mut report = WipeReport { removed_saves: 0, kept_saves: Vec::new(), firewall: FirewallOutcome::NotAsked };
    if delete_saves {
        match saves {
            Some(s) => {
                for r in existing_saves(s, &war_saves)? {
                    match s.delete(&r.id) {
                        Ok(()) => report.removed_saves += 1,
                        Err(_) => report.kept_saves.push(r.id),
                    }
                }
            }
            None => report.kept_saves = war_saves,
        }
    }
    if firewall {
        let rules = remove_rules(m, &firewall_programs(m, data, &state));
        let ports = m.opened_ports(data);
        report.firewall = if ports == 0 { rules } else { with_ports(rules, ports, m.close_ports(data)) };
    }
    remove_tree(&dir(data))?;
    if let Some(g) = game_side {
        match std::fs::remove_file(g.join(NAMES_FILE)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    Ok(report)
}

/// The outcome of the Windows rules joined with closing `ports` LAN port entries in a Linux firewall.
fn with_ports(rules: FirewallOutcome, ports: u32, closed: Elevated) -> FirewallOutcome {
    match closed {
        Elevated::Declined => FirewallOutcome::Declined,
        Elevated::Failed(message) => FirewallOutcome::Failed { message },
        Elevated::Done => match rules {
            FirewallOutcome::NotAsked | FirewallOutcome::None => FirewallOutcome::Removed { count: ports },
            FirewallOutcome::Removed { count } => FirewallOutcome::Removed { count: count + ports },
            other => other,
        },
    }
}

fn remove_rules(m: &dyn Machine, programs: &[PathBuf]) -> FirewallOutcome {
    let count = || m.firewall_rules().map(|t| count_all(&t, programs));
    let Some(before) = count() else { return FirewallOutcome::Failed { message: "the firewall rules couldn't be read".into() } };
    if before == 0 {
        return FirewallOutcome::None;
    }
    // Only the programs that have rules, so netsh has something to delete for each.
    let with_rules: Vec<PathBuf> = m.firewall_rules().map_or_else(|| programs.to_vec(), |t| programs.iter().filter(|p| count_rules(&t, p) > 0).cloned().collect());
    let ran = m.delete_firewall_rules(&with_rules);
    if ran == Elevated::Declined {
        return FirewallOutcome::Declined;
    }
    let left = count().unwrap_or(if ran == Elevated::Done { 0 } else { before });
    if left < before {
        return FirewallOutcome::Removed { count: before - left };
    }
    match ran {
        Elevated::Failed(message) => FirewallOutcome::Failed { message },
        _ => FirewallOutcome::Failed { message: "the rules are still there".into() },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::commander::{Installed, WarRef, forget, play_war};
    use crate::saves::test_support::{ACCOUNT, World, world};
    use crate::testutil::put;

    #[test]
    fn linux_ports_join_the_firewall_outcome() {
        assert_eq!(with_ports(FirewallOutcome::None, 2, Elevated::Done), FirewallOutcome::Removed { count: 2 });
        assert_eq!(with_ports(FirewallOutcome::Removed { count: 1 }, 2, Elevated::Done), FirewallOutcome::Removed { count: 3 });
        assert_eq!(with_ports(FirewallOutcome::None, 2, Elevated::Declined), FirewallOutcome::Declined);
        assert_eq!(with_ports(FirewallOutcome::None, 2, Elevated::Failed("x".into())), FirewallOutcome::Failed { message: "x".into() });
    }

    /// netsh as a fake: the rule text, what deleting them does, and whether the server runs.
    struct Fake {
        rules: RefCell<Option<Vec<u8>>>,
        delete: Elevated,
        server_runs: bool,
        deletes: RefCell<Vec<PathBuf>>,
        asked: Cell<u32>,
        own: Option<PathBuf>,
    }

    impl Fake {
        fn new(rules: Option<String>, delete: Elevated) -> Fake {
            Fake { rules: RefCell::new(rules.map(String::into_bytes)), delete, server_runs: false, deletes: RefCell::new(Vec::new()), asked: Cell::new(0), own: None }
        }
    }

    impl Machine for Fake {
        fn firewall_rules(&self) -> Option<Vec<u8>> {
            self.asked.set(self.asked.get() + 1);
            self.rules.borrow().clone()
        }
        fn own_exe(&self) -> Option<PathBuf> {
            self.own.clone()
        }
        fn delete_firewall_rules(&self, programs: &[PathBuf]) -> Elevated {
            self.deletes.borrow_mut().extend(programs.iter().cloned());
            if self.delete == Elevated::Done {
                *self.rules.borrow_mut() = Some(b"\r\nNo rules match the specified criteria.\r\n".to_vec());
            }
            self.delete.clone()
        }
        fn running(&self, exe: &str) -> bool {
            self.server_runs && exe == DEFAULT_SERVER
        }
    }

    /// netsh text in Portuguese, with two rules for `exe` (as Windows writes them: lower case
    /// but the drive) and one for another program.
    fn rules_for(exe: &Path) -> String {
        let p = exe.display().to_string();
        let lower = format!("{}{}", &p[..1], p[1..].to_lowercase());
        let rule = |name: &str, prog: &str| format!("\r\nNome da Regra:                        {name}\r\n----------\r\nDescri\u{87}\u{c6}o:     {name}\r\nPrograma:                             {prog}\r\nA\u{87}\u{c6}o:     Permitir\r\n");
        rule("commander-server.exe", &lower) + &rule("commander-server.exe", &lower.replace('\\', "/")) + &rule("obsidian.exe", "C:\\other\\obsidian.exe")
    }

    fn local_war() -> WarRef {
        WarRef { id: "aaa".into(), name: "Home War".into(), host: WarHost::Local, base: None, save: None, computer: None, model: None, key: None, ..Default::default() }
    }

    fn lan_war() -> WarRef {
        WarRef { id: "k3x9q2".into(), name: "Night War".into(), host: WarHost::Lan, base: Some("http://192.168.0.7:17494".into()), save: None, computer: None, model: None, key: None, ..Default::default() }
    }

    /// A PC that installed Commander, hosts two wars, played a local and a LAN war (the LAN one
    /// selected) and has a dedicated Sandbox. Returns the game-side folder.
    fn played(w: &World) -> PathBuf {
        let data = &w.data;
        let s = w.saves();
        s.list().unwrap();
        s.create_sandbox("Mine").unwrap();
        let pkg = package::package_dir(data);
        put(&pkg, DEFAULT_SERVER, b"MZ");
        put(&pkg, "ui/index.html", b"<html>");
        let wars = server::server_data(data).join("wars");
        put(&wars, "aaa/war.json", b"{}");
        put(&wars, "bbb/war.json", b"{}");
        put(&wars, "broken/feed.jsonl", b"");
        let state = State { package: Some(Installed { version: "0.3.0".into(), protocol: 1, server: DEFAULT_SERVER.into(), ui: "ui".into() }), ..Default::default() };
        state.save(data).unwrap();
        play_war(data, &s, local_war(), None).unwrap();
        play_war(data, &s, lan_war(), None).unwrap();
        let side = crate::settings::game_side_dir(&w.game.saves_dir);
        put(&side, NAMES_FILE, b"region_bugs_01\tPolaris\n");
        side
    }

    fn settings_war(side: &Path) -> Option<crate::commander::WarLine> {
        crate::settings::parse(&std::fs::read_to_string(side.join(crate::settings::FILE)).unwrap_or_default()).war
    }

    fn sandboxes(w: &World) -> Vec<String> {
        w.saves().list().unwrap().saves.into_iter().filter(|r| r.kind == SaveKind::Sandbox).map(|r| r.name).collect()
    }

    #[test]
    fn the_footprint_counts_what_commander_left() {
        let w = world();
        let side = played(&w);
        let fake = Fake::new(Some(rules_for(&server_exe(&w.data, &State::load(&w.data)))), Elevated::Done);
        let f = footprint_on(&fake, &w.data, Some(&w.saves()), Some(&side));
        assert_eq!(f.package.as_deref(), Some("0.3.0"));
        assert_eq!(f.hosted, 2, "a folder without war.json isn't a war");
        assert_eq!(f.joined, 1);
        assert_eq!(f.saves.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Home War", "Night War"]);
        assert!(f.names_file);
        assert_eq!(f.firewall_rules, 2);
        assert!(f.bytes > 0);
        let json = serde_json::to_value(&f).unwrap();
        assert!(json.get("namesFile").is_some() && json.get("firewallRules").is_some());

        let none = footprint_on(&Fake::new(None, Elevated::Done), &w.data, None, None);
        assert!(none.saves.is_empty() && !none.names_file && none.firewall_rules == 0);
    }

    #[test]
    fn a_wipe_goes_back_to_official_and_removes_everything() {
        let w = world();
        let side = played(&w);
        assert!(settings_war(&side).is_some());
        let exe = server_exe(&w.data, &State::load(&w.data));
        let fake = Fake::new(Some(rules_for(&exe)), Elevated::Done);
        let r = wipe_on(&fake, &w.data, Some(&w.saves()), Some(&side), true, true).unwrap();
        assert_eq!(r, WipeReport { removed_saves: 2, kept_saves: vec![], firewall: FirewallOutcome::Removed { count: 2 } });
        assert_eq!(*fake.deletes.borrow(), vec![exe]);
        assert!(!dir(&w.data).exists());
        assert!(!side.join(NAMES_FILE).exists());
        assert_eq!(sandboxes(&w), ["Mine"], "only the war saves go");
        let list = w.saves().list().unwrap();
        assert!(list.saves.iter().any(|r| r.kind == SaveKind::Official && r.active), "the Official save is active again");
        assert_eq!(crate::saves::active_sandbox(&w.data), None);
        assert_eq!(settings_war(&side), None, "the settings file lost the war line");
        let after = footprint_on(&fake, &w.data, Some(&w.saves()), Some(&side));
        assert_eq!(after, Footprint { package: None, hosted: 0, joined: 0, saves: vec![], names_file: false, firewall_rules: 0, bytes: 0 });
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json, serde_json::json!({"removedSaves": 2, "keptSaves": [], "firewall": {"kind": "removed", "count": 2}}));
    }

    #[test]
    fn saves_and_rules_stay_unless_asked_and_a_declined_prompt_is_reported() {
        let w = world();
        let side = played(&w);
        let fake = Fake::new(Some(rules_for(&server_exe(&w.data, &State::load(&w.data)))), Elevated::Declined);
        let r = wipe_on(&fake, &w.data, Some(&w.saves()), Some(&side), false, true).unwrap();
        assert_eq!(r, WipeReport { removed_saves: 0, kept_saves: vec![], firewall: FirewallOutcome::Declined });
        assert_eq!(sandboxes(&w), ["Mine", "Home War", "Night War"]);
        assert_eq!(crate::saves::active_sandbox(&w.data), None, "back to the Official war all the same");
        assert!(!dir(&w.data).exists());
        assert_eq!(serde_json::to_value(&r.firewall).unwrap(), serde_json::json!({"kind": "declined"}));

        // Nothing left: the firewall unasked, or asked with no rules.
        let quiet = Fake::new(Some(String::new()), Elevated::Done);
        assert_eq!(wipe_on(&quiet, &w.data, Some(&w.saves()), Some(&side), true, false).unwrap().firewall, FirewallOutcome::NotAsked);
        assert_eq!(quiet.asked.get(), 0, "netsh isn't asked when the firewall isn't part of the wipe");
        assert_eq!(wipe_on(&quiet, &w.data, Some(&w.saves()), Some(&side), true, true).unwrap().firewall, FirewallOutcome::None);
        assert!(quiet.deletes.borrow().is_empty());
        let failed = Fake::new(Some(rules_for(&server_exe(&w.data, &State::load(&w.data)))), Elevated::Failed("netsh ended with code 1".into()));
        assert_eq!(wipe_on(&failed, &w.data, None, None, false, true).unwrap().firewall, FirewallOutcome::Failed { message: "netsh ended with code 1".into() });
    }

    #[test]
    fn a_war_save_left_selected_goes_when_the_saves_do() {
        let w = world();
        let side = played(&w);
        // Forgetting the war clears the selection and leaves the war save selected.
        forget(&w.data, "k3x9q2").unwrap();
        let mut state = State::load(&w.data);
        state.upsert(WarRef { save: crate::saves::active_sandbox(&w.data), ..lan_war() });
        state.save(&w.data).unwrap();
        assert!(State::load(&w.data).playing.is_none());
        let r = wipe_on(&Fake::new(None, Elevated::Done), &w.data, Some(&w.saves()), Some(&side), true, false).unwrap();
        assert_eq!((r.removed_saves, r.kept_saves.len()), (2, 0));
        assert_eq!(crate::saves::active_sandbox(&w.data), None);
    }

    #[test]
    fn without_saves_the_war_is_cleared_and_its_saves_listed_as_kept() {
        let w = world();
        let side = played(&w);
        let ids = war_saves(&State::load(&w.data));
        let r = wipe_on(&Fake::new(None, Elevated::Done), &w.data, None, Some(&side), true, false).unwrap();
        assert_eq!(r.kept_saves, ids);
        assert_eq!(sandboxes(&w), ["Mine", "Home War", "Night War"]);
        assert!(!dir(&w.data).exists());
    }

    #[test]
    fn a_running_game_or_server_refuses_the_wipe() {
        let w = world();
        let side = played(&w);
        let busy = Saves::for_account(&w.data, &w.game, ACCOUNT, || true);
        let fake = Fake::new(None, Elevated::Done);
        assert!(matches!(wipe_on(&fake, &w.data, Some(&busy), Some(&side), true, true), Err(ObsidianError::GameRunning)));
        let serving = Fake { server_runs: true, ..Fake::new(None, Elevated::Done) };
        assert!(matches!(wipe_on(&serving, &w.data, Some(&w.saves()), Some(&side), true, true), Err(ObsidianError::CommanderRunning)));
        assert!(dir(&w.data).join("state.json").is_file() && side.join(NAMES_FILE).is_file(), "nothing removed");
        assert!(State::load(&w.data).playing.is_some());
        assert!(fake.deletes.borrow().is_empty() && serving.deletes.borrow().is_empty());
    }

    #[test]
    fn rules_are_found_in_any_language_case_and_code_page() {
        let exe = Path::new(r"C:\Users\João\AppData\Local\ObsidianLauncher\commander\package\commander-server.exe");
        // Portuguese labels in code page 850, where "ã" is one byte.
        let mut oem = b"Nome da Regra:  commander-server.exe\r\nPrograma:   C:\\users\\jo".to_vec();
        oem.push(0xc6);
        oem.extend_from_slice(b"o\\appdata\\local\\obsidianlauncher\\commander\\package\\commander-server.exe\r\n");
        assert_eq!(count_rules(&oem, exe), 1);
        let utf8 = "Program:  c:/users/joão/appdata/local/obsidianlauncher/commander/package/commander-server.exe\n";
        assert_eq!(count_rules(utf8.as_bytes(), exe), 1);
        assert_eq!(count_rules(b"Rule Name: commander-server.exe\nProgram: C:\\other\\commander-server.exe\n", exe), 0);
        assert_eq!(count_rules(b"", exe), 0);
    }

    // System32 paths only exist on Windows.
    #[cfg(windows)]
    #[test]
    fn the_admin_script_quotes_each_path_and_travels_encoded() {
        assert_eq!([base64(b""), base64(b"M"), base64(b"Ma"), base64(b"Man"), base64(b"Many")], ["", "TQ==", "TWE=", "TWFu", "TWFueQ=="]);
        let programs = [PathBuf::from(r"C:\Users\O'Neil\x\commander-server.exe"), PathBuf::from(r"C:\Program Files\Tom & Jerry\Obsidian.exe")];
        let inner = admin_delete_script(&programs);
        assert!(inner.contains(r"@('C:\Users\O''Neil\x\commander-server.exe', 'C:\Program Files\Tom & Jerry\Obsidian.exe')"), "{inner}");
        let tool = system_tool("netsh.exe");
        assert!(tool.is_absolute() && tool.ends_with(r"System32\netsh.exe"), "{tool:?}");
        assert!(inner.contains(&format!("& '{}' advfirewall firewall delete rule name=all \"program=$p\"", tool.display())), "netsh by full path: {inner}");
        let s = elevated_delete_script(&programs);
        assert!(s.contains("$i.Verb = 'runas'") && s.contains("exit 1223"));
        assert!(s.contains(r"WindowsPowerShell\v1.0\powershell.exe'"), "PowerShell by full path: {s}");
        let args = s.lines().find(|l| l.starts_with("$i.Arguments")).unwrap();
        let encoded = args.rsplit(' ').next().unwrap().trim_end_matches('\'');
        assert!(encoded.len() > 100 && encoded.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)), "no path in the command line: {args}");
    }

    #[test]
    fn the_rules_of_this_obsidian_go_with_the_ones_of_the_server() {
        let w = world();
        let side = played(&w);
        let server = server_exe(&w.data, &State::load(&w.data));
        let own = PathBuf::from(r"C:\Program Files\Obsidian Launcher\Obsidian.exe");
        let rule = |prog: &str| format!("\r\nRule Name: x\r\nProgram:    {prog}\r\n");
        let rules = rule(&server.display().to_string()) + &rule(r"c:\program files\obsidian launcher\obsidian.exe") + &rule(r"C:\other\obsidian.exe");
        let fake = Fake { own: Some(own.clone()), ..Fake::new(Some(rules.clone()), Elevated::Done) };
        assert_eq!(footprint_on(&fake, &w.data, None, None).firewall_rules, 2, "the server and this Obsidian, not another copy");
        let r = wipe_on(&fake, &w.data, Some(&w.saves()), Some(&side), false, true).unwrap();
        assert_eq!(r.firewall, FirewallOutcome::Removed { count: 2 });
        assert_eq!(*fake.deletes.borrow(), vec![server, own.clone()]);

        // Only Obsidian has rules: netsh is asked about that one alone.
        let w = world();
        played(&w);
        let fake = Fake { own: Some(own.clone()), ..Fake::new(Some(rule(r"C:\Program Files\Obsidian Launcher\Obsidian.exe")), Elevated::Done) };
        assert_eq!(wipe_on(&fake, &w.data, None, None, false, true).unwrap().firewall, FirewallOutcome::Removed { count: 1 });
        assert_eq!(*fake.deletes.borrow(), vec![own]);
    }
}
