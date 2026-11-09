//! The game-side settings file the loader reads at boot: consent, the guardrails, option values
//! and keys. A change applies at the next game launch, with no redeploy.

use std::path::{Path, PathBuf};

use crate::Result;

pub const FILE: &str = "settings.txt";
/// The name of every folder Obsidian makes on the PC.
pub const DIR: &str = "ObsidianLauncher";
/// What the folders were named before the product became Obsidian Launcher.
pub const LEGACY_DIR: &str = "ObsidianModManager";
/// In the game-side folder while a loader deployed before the rename may still read the old folder:
/// the next deploy settles it.
pub(crate) const CARRY_OVER: &str = "legacy-sync.pending";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub consent: bool,
    pub block_achievements: bool,
    pub freeze_progress: bool,
    /// The selected Sandbox save, when one is. `block_achievements` is forced on for the file while
    /// one is, so that a Sandbox never earns an achievement.
    pub sandbox: Option<String>,
    /// The custom war this launch plays, with the Sandbox above as its save. Never without one.
    pub war: Option<crate::commander::WarLine>,
    /// Leave client mods out of the lobby info this player hosts.
    pub hide_client_mods: bool,
    /// (mod id, option id, option type, value text)
    pub options: Vec<(String, String, String, String)>,
    /// (mod id, keybind id, key name)
    pub keys: Vec<(String, String, String)>,
}

/// `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher`, beside the game's own save folder. Folders
/// are named ObsidianLauncher, not Obsidian, which is the name of other software.
///
/// The first call after the rename copies the folder an older copy made (`ObsidianModManager`:
/// logs, `.store` files, settings) to the new name. It is a copy and not a move, because a loader
/// deployed before the rename still reads and writes the old folder until the next deploy.
pub fn game_side_dir(saves_dir: &Path) -> PathBuf {
    let parent = saves_dir.parent().unwrap_or(saves_dir);
    let dir = parent.join(DIR);
    carry_over(&parent.join(LEGACY_DIR), &dir);
    dir
}

/// Copies `old` to `new` when only `old` exists, through a folder beside `new` that takes the name
/// when the copy is whole, so a copy cut short is made again instead of kept. Failing is fine: the
/// loader makes the folder again.
fn carry_over(old: &Path, new: &Path) {
    if new.exists() || !old.is_dir() {
        return;
    }
    let tmp = crate::paths::with_suffix(new, ".copying");
    let _ = std::fs::remove_dir_all(&tmp);
    let done = crate::paths::copy_tree(old, &tmp, false)
        .and_then(|_| std::fs::write(tmp.join(CARRY_OVER), b""))
        .and_then(|_| crate::paths::rename_retrying(&tmp, new));
    if done.is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

/// The folder an older copy made, while `dir` is waiting for the deploy that settles the rename.
fn pending_legacy(dir: &Path) -> Option<PathBuf> {
    if dir.file_name()? != std::ffi::OsStr::new(DIR) || !dir.join(CARRY_OVER).is_file() {
        return None;
    }
    Some(dir.with_file_name(LEGACY_DIR)).filter(|old| old.is_dir())
}

/// Called by a deploy, which puts a loader that reads `dir` in the game: takes what the older loader
/// wrote to the old folder since the copy (a file is taken only when it is newer), then stops
/// keeping the old folder up to date. The old folder is left where it is.
pub fn settle_carry_over(dir: &Path) {
    if let Some(old) = pending_legacy(dir) {
        let _ = crate::paths::copy_tree(&old, dir, true);
    }
    let _ = std::fs::remove_file(dir.join(CARRY_OVER));
}

/// A value on one line: a tab or a line break in it would start a field or a line of its own, and a
/// shared profile could write a `consent` line that way.
fn field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

pub fn render(s: &Settings) -> String {
    let mut out = String::from("# Written by Obsidian. Changes made here are replaced at the next deploy.\n");
    out.push_str(&format!("consent\t{}\n", if s.consent { 1 } else { 0 }));
    out.push_str(&format!("block_achievements\t{}\n", if s.block_achievements || s.sandbox.is_some() { 1 } else { 0 }));
    out.push_str(&format!("freeze_progress\t{}\n", if s.freeze_progress { 1 } else { 0 }));
    // Absent means the Official save.
    if let Some(id) = &s.sandbox {
        out.push_str(&format!("sandbox\t{}\n", field(id)));
        // A war only ever with a Sandbox as its save: the loader refuses war mode without one.
        if let Some(w) = &s.war {
            out.push_str(&format!("war\t{}\t{}\t{}\n", field(&w.id), field(&w.url), field(&w.pool)));
        }
    }
    if s.hide_client_mods {
        out.push_str("hide_client_mods\t1\n");
    }
    for (m, o, t, v) in &s.options {
        out.push_str(&format!("option\t{}\t{}\t{}\t{}\n", field(m), field(o), field(t), field(v)));
    }
    for (m, k, key) in &s.keys {
        out.push_str(&format!("key\t{}\t{}\t{}\n", field(m), field(k), field(key)));
    }
    out
}

/// Lines that don't parse are skipped, as the loader does.
pub fn parse(text: &str) -> Settings {
    let mut s = Settings::default();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["consent", v] => s.consent = *v == "1",
            ["block_achievements", v] => s.block_achievements = *v == "1",
            ["freeze_progress", v] => s.freeze_progress = *v == "1",
            ["sandbox", id] => s.sandbox = Some(id.to_string()),
            ["war", id, url, pool] => s.war = Some(crate::commander::WarLine { id: id.to_string(), url: url.to_string(), pool: pool.to_string() }),
            ["hide_client_mods", v] => s.hide_client_mods = *v == "1",
            ["option", m, o, t, v] => s.options.push((m.to_string(), o.to_string(), t.to_string(), v.to_string())),
            ["key", m, k, key] => s.keys.push((m.to_string(), k.to_string(), key.to_string())),
            _ => {}
        }
    }
    s
}

/// Writes the file atomically and makes sure the logs folder exists for the loader.
pub fn write(dir: &Path, s: &Settings) -> Result<()> {
    std::fs::create_dir_all(dir.join("logs"))?;
    let text = render(s);
    crate::paths::write_atomic(&dir.join(FILE), text.as_bytes())?;
    // A loader deployed before the rename reads the old folder, and a guardrail must reach it.
    if let Some(old) = pending_legacy(dir) {
        let _ = crate::paths::write_atomic(&old.join(FILE), text.as_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `saves`, with an older copy's game-side folder beside it holding a log and a mod's store.
    fn older_copy() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let t = crate::testutil::spaced_tempdir();
        let saves = t.path().join("Arrowhead/Helldivers/saves");
        let old = t.path().join("Arrowhead/Helldivers").join(LEGACY_DIR);
        crate::testutil::put(&old, "logs/loader.log", b"old log");
        crate::testutil::put(&old, "logs/a.b.store", b"store");
        crate::testutil::put(&old, FILE, b"consent\t1\n");
        (t, saves, old)
    }

    #[test]
    fn the_game_side_folder_of_an_older_copy_is_copied_not_moved() {
        let (_t, saves, old) = older_copy();
        let dir = game_side_dir(&saves);
        assert_eq!(dir.file_name().unwrap(), DIR);
        assert_eq!(std::fs::read(dir.join("logs/a.b.store")).unwrap(), b"store");
        assert_eq!(std::fs::read(dir.join("logs/loader.log")).unwrap(), b"old log");
        assert_eq!(std::fs::read(dir.join(FILE)).unwrap(), b"consent\t1\n");
        assert!(old.join("logs/a.b.store").is_file() && old.join(FILE).is_file(), "an older loader still reads it");
        assert!(!crate::paths::with_suffix(&dir, ".copying").exists());
        // A second call, and a folder that has changed since, are left as they are.
        std::fs::write(dir.join("logs/a.b.store"), b"new store").unwrap();
        game_side_dir(&saves);
        assert_eq!(std::fs::read(dir.join("logs/a.b.store")).unwrap(), b"new store");
    }

    #[test]
    fn no_older_game_side_folder_makes_nothing() {
        let t = crate::testutil::spaced_tempdir();
        let saves = t.path().join("Arrowhead/Helldivers/saves");
        let dir = game_side_dir(&saves);
        assert!(!dir.exists() && !dir.with_file_name(LEGACY_DIR).exists());
    }

    #[test]
    fn until_a_deploy_settles_it_settings_reach_the_older_folder_too() {
        let (_t, saves, old) = older_copy();
        let dir = game_side_dir(&saves);
        let s = Settings { block_achievements: true, ..Default::default() };
        write(&dir, &s).unwrap();
        assert_eq!(parse(&std::fs::read_to_string(old.join(FILE)).unwrap()), s, "an older loader gets the guardrail");
        // The older loader keeps writing to the old folder until the deploy.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        std::fs::write(old.join("logs/a.b.store"), b"written meanwhile").unwrap();
        std::fs::File::options().write(true).open(old.join("logs/a.b.store")).unwrap().set_modified(later).unwrap();
        settle_carry_over(&dir);
        assert_eq!(std::fs::read(dir.join("logs/a.b.store")).unwrap(), b"written meanwhile");
        assert!(!dir.join(CARRY_OVER).exists());
        // Settled: the old folder is no longer written, and a store deleted since stays deleted.
        write(&dir, &Settings::default()).unwrap();
        assert!(parse(&std::fs::read_to_string(old.join(FILE)).unwrap()).block_achievements);
        std::fs::remove_file(dir.join("logs/a.b.store")).unwrap();
        settle_carry_over(&dir);
        assert!(!dir.join("logs/a.b.store").exists());
    }

    #[test]
    fn a_value_cant_write_a_line_of_its_own() {
        let s = Settings { keys: vec![("a.b".into(), "k".into(), "f8\nconsent\t1".into())], ..Default::default() };
        let back = parse(&render(&s));
        assert!(!back.consent);
        assert_eq!(back.keys[0].2, "f8 consent 1");
    }

    #[test]
    fn settings_file_round_trips() {
        let s = Settings {
            consent: true,
            block_achievements: true,
            freeze_progress: true,
            sandbox: Some("0a1b2c3d".into()),
            war: Some(crate::commander::WarLine { id: "w1".into(), url: "http://127.0.0.1:1/t/1.0/".into(), pool: "abcdef012345".into() }),
            hide_client_mods: true,
            options: vec![("a.b".into(), "d".into(), "number".into(), "2.5".into())],
            keys: vec![("a.b".into(), "toggle".into(), "f8".into())],
        };
        let t = crate::testutil::spaced_tempdir();
        write(t.path(), &s).unwrap();
        assert!(t.path().join("logs").is_dir());
        assert_eq!(parse(&std::fs::read_to_string(t.path().join(FILE)).unwrap()), s);
    }

    #[test]
    fn a_sandbox_forces_block_achievements_and_absent_lines_stay_out() {
        let plain = Settings::default();
        assert!(!render(&plain).contains("sandbox") && !render(&plain).contains("hide_client"));
        let s = Settings { sandbox: Some("0a1b2c3d".into()), ..Default::default() };
        let text = render(&s);
        assert!(text.contains("block_achievements\t1\n") && text.contains("sandbox\t0a1b2c3d\n"));
    }

    #[test]
    fn a_war_line_rides_only_with_its_sandbox_and_round_trips() {
        let war = crate::commander::WarLine { id: "k3x9q2".into(), url: "http://127.0.0.1:40000/abc/1.0/".into(), pool: "0123456789ab".into() };
        let alone = Settings { war: Some(war.clone()), ..Default::default() };
        assert!(!render(&alone).contains("war\t"), "no Sandbox, no war line");
        let s = Settings { sandbox: Some("sb1".into()), war: Some(war), ..Default::default() };
        let text = render(&s);
        assert!(text.contains("war\tk3x9q2\thttp://127.0.0.1:40000/abc/1.0/\t0123456789ab\n"), "{text}");
        assert_eq!(parse(&text).war, s.war);
    }

    #[test]
    fn game_side_dir_is_beside_the_saves() {
        assert_eq!(game_side_dir(Path::new("C:/x/Arrowhead/Helldivers/saves")), Path::new("C:/x/Arrowhead/Helldivers/ObsidianLauncher"));
    }
}
