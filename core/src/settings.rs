//! The game-side settings file the loader reads at boot: consent, the guardrails, option values
//! and keys. A change applies at the next game launch, with no redeploy.

use std::path::{Path, PathBuf};

use crate::Result;

pub const FILE: &str = "settings.txt";
/// The name of every folder Obsidian makes on the PC.
pub const DIR: &str = "ObsidianModManager";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub consent: bool,
    pub block_achievements: bool,
    pub freeze_progress: bool,
    /// (mod id, option id, option type, value text)
    pub options: Vec<(String, String, String, String)>,
    /// (mod id, keybind id, key name)
    pub keys: Vec<(String, String, String)>,
}

/// `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager`, beside the game's own save folder. Folders
/// are named ObsidianModManager, not Obsidian, which is the name of other software.
pub fn game_side_dir(saves_dir: &Path) -> PathBuf {
    saves_dir.parent().map(|p| p.join(DIR)).unwrap_or_else(|| saves_dir.join(DIR))
}

/// A value on one line: a tab or a line break in it would start a field or a line of its own, and a
/// shared profile could write a `consent` line that way.
fn field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

pub fn render(s: &Settings) -> String {
    let mut out = String::from("# Written by Obsidian. Changes made here are replaced at the next deploy.\n");
    out.push_str(&format!("consent\t{}\n", if s.consent { 1 } else { 0 }));
    out.push_str(&format!("block_achievements\t{}\n", if s.block_achievements { 1 } else { 0 }));
    out.push_str(&format!("freeze_progress\t{}\n", if s.freeze_progress { 1 } else { 0 }));
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
    crate::paths::write_atomic(&dir.join(FILE), render(s).as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            options: vec![("a.b".into(), "d".into(), "number".into(), "2.5".into())],
            keys: vec![("a.b".into(), "toggle".into(), "f8".into())],
        };
        let t = crate::testutil::spaced_tempdir();
        write(t.path(), &s).unwrap();
        assert!(t.path().join("logs").is_dir());
        assert_eq!(parse(&std::fs::read_to_string(t.path().join(FILE)).unwrap()), s);
    }

    #[test]
    fn game_side_dir_is_beside_the_saves() {
        assert_eq!(game_side_dir(Path::new("C:/x/Arrowhead/Helldivers/saves")), Path::new("C:/x/Arrowhead/Helldivers/ObsidianModManager"));
    }
}
