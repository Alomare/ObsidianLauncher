//! The text a player pastes into a bug report: versions, what is on, what the last deploy did and the
//! end of the loader log. Paths lose the Windows account name, since these end up in public threads.

use std::fmt::Write;
use std::path::Path;

use crate::Result;
use crate::deploy::{self, loader_log};
use crate::game::GameInstall;
use crate::library::{Library, Profile};

const LOG_LINES: usize = 60;

/// What only the caller knows: the app's version, and whether it is a portable copy.
pub struct AppInfo<'a> {
    pub version: &'a str,
    pub portable: bool,
}

/// `account` is the Windows account name, which is taken out of every path.
fn without_account(text: String, account: Option<&str>) -> String {
    let Some(name) = account.filter(|n| !n.is_empty()) else { return text };
    ["\\", "/"].iter().fold(text, |t, sep| t.replace(&format!("{sep}Users{sep}{name}"), &format!("{sep}Users{sep}<user>")))
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

pub fn build(data: &Path, game: Option<&GameInstall>, app: &AppInfo) -> Result<String> {
    let mut out = String::new();
    let kind = if app.portable { "portable" } else { "installed" };
    let _ = writeln!(out, "Obsidian {} ({kind}), {}", app.version, std::env::consts::OS);

    let lib = Library::open(data)?;
    let mut profile = Profile::load(data)?;
    profile.sync(&lib)?;
    let manifests = lib.list()?;
    let _ = writeln!(out, "Session mods from other players: {}", if profile.block_public { "blocked in public lobbies" } else { "allowed" });

    let on: Vec<_> = profile.mods.iter().filter(|e| e.enabled).collect();
    let _ = writeln!(out, "Mods on, in load order ({} of {}):", on.len(), profile.mods.len());
    for (i, e) in on.iter().enumerate() {
        match manifests.iter().find(|m| m.id == e.id) {
            Some(m) => {
                let _ = writeln!(out, "  {}. {} {}", i + 1, m.id, m.version);
            }
            None => {
                let _ = writeln!(out, "  {}. {} (not in the library)", i + 1, e.id);
            }
        }
    }

    match game {
        None => out.push_str("Game: not found\n"),
        Some(g) => {
            let _ = writeln!(out, "Game: {}", g.root.display());
            match deploy::load_state(data)? {
                Some(s) => {
                    let _ = writeln!(out, "Last deploy: {}, {} mods, build {}", s.time, s.mods.len(), s.build);
                }
                None => out.push_str("Last deploy: none\n"),
            }
            if let Some(id) = deploy::crash_culprit(g) {
                let _ = writeln!(out, "The last launch stopped while loading {id}");
            }
            let log = loader_log(g);
            match std::fs::read_to_string(&log) {
                Ok(text) => {
                    if deploy::unclean_end(g).is_some() {
                        out.push_str("The last launch did not close normally\n");
                    }
                    let _ = write!(out, "\n--- loader.log, last {LOG_LINES} lines ---\n{}\n", tail(&text, LOG_LINES));
                }
                Err(_) => out.push_str("No loader log yet\n"),
            }
        }
    }
    Ok(without_account(out, std::env::var("USERNAME").ok().as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_lose_the_account_name_and_nothing_else() {
        let text = "Game: C:\\Users\\Ana\\Games\\HD and c:/users/x and D:/Users/Ana/mods, Ana said".to_string();
        assert_eq!(without_account(text, Some("Ana")), "Game: C:\\Users\\<user>\\Games\\HD and c:/users/x and D:/Users/<user>/mods, Ana said");
        assert_eq!(without_account("C:\\Users\\Ana".into(), None), "C:\\Users\\Ana");
        assert_eq!(without_account("C:\\Users\\Ana".into(), Some("")), "C:\\Users\\Ana");
    }

    #[test]
    fn tail_keeps_the_end_of_the_log() {
        assert_eq!(tail("a\nb\nc\n", 2), "b\nc");
        assert_eq!(tail("a\n", 5), "a");
        assert_eq!(tail("", 5), "");
    }

    #[test]
    fn a_report_lists_what_is_on_with_no_game_found() {
        let t = crate::testutil::spaced_tempdir();
        crate::starter::seed(t.path()).unwrap();
        let r = build(t.path(), None, &AppInfo { version: "9.9.9", portable: true }).unwrap();
        assert!(r.starts_with("Obsidian 9.9.9 (portable)"), "{r}");
        assert!(r.contains("Mods on, in load order (2 of 3):\n  1. obsidian.api"), "{r}");
        assert!(r.contains("obsidian.qol"), "{r}");
        assert!(r.contains("Game: not found"), "{r}");
    }
}
