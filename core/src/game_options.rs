//! Option values the player changed in the game's pause menu. Obsidian API keeps them in a storage
//! file as `option/<mod id>/<option id>` until the app takes them into the active profile, so the mod
//! details and the pause menu show the same values.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::Result;
use crate::edit::{set_option_of, write_settings};
use crate::library::{Library, Profile};
use crate::manifest::Manifest;
use crate::paths::{rename_retrying, with_suffix, write_through};
use crate::settings::game_side_dir;

/// Obsidian API's storage file, in the logs folder beside the mods' logs.
pub const STORE: &str = "obsidianapi.store";
/// What the takes did, for whoever wonders why a value didn't come over.
pub const LOG: &str = "game-options.log";
const PREFIX: &str = "option/";
const LOG_LINES: usize = 200;

/// One line of a storage file, as the loader reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreLine {
    pub key: String,
    /// `s`, `b` or `n`.
    pub kind: char,
    pub value: String,
}

/// Undoes the loader's escapes. An unknown escape stays as written, as in the loader.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// `key<TAB>type<TAB>value`, or `None` for a line the loader skips. Keys and string values lose
/// their escapes; booleans and numbers stay as written, as the loader reads them.
pub fn parse_line(line: &str) -> Option<StoreLine> {
    let (key, rest) = line.split_once('\t')?;
    let (kind, value) = rest.split_once('\t')?;
    let kind = match kind {
        "s" => 's',
        "b" => 'b',
        "n" => 'n',
        _ => return None,
    };
    let value = if kind == 's' { unescape(value) } else { value.to_string() };
    Some(StoreLine { key: unescape(key), kind, value })
}

/// The value as the app's own controls send it. A whole number stays whole, so the settings file
/// gets `4`, not `4.0`.
fn json_value(l: &StoreLine) -> Option<Value> {
    match l.kind {
        'b' => match l.value.as_str() {
            "1" => Some(Value::Bool(true)),
            "0" => Some(Value::Bool(false)),
            _ => None,
        },
        's' => Some(Value::String(l.value.clone())),
        _ => {
            let n = l.value.trim().parse::<f64>().ok().filter(|n| n.is_finite())?;
            if n.fract() == 0.0 && n.abs() < 9.0e15 { Some(Value::from(n as i64)) } else { serde_json::Number::from_f64(n).map(Value::Number) }
        }
    }
}

/// What a take did.
#[derive(Debug, Default, PartialEq)]
pub struct Taken {
    /// (mod id, option id, value) now in the active profile.
    pub taken: Vec<(String, String, Value)>,
    /// (key, value as written, why), left in the file.
    pub skipped: Vec<(String, String, String)>,
}

/// Where Obsidian API keeps the values of the game whose saves are in `saves_dir`.
pub fn store_path(saves_dir: &Path) -> PathBuf {
    game_side_dir(saves_dir).join("logs").join(STORE)
}

fn take_one(manifests: &BTreeMap<String, Manifest>, profile: &mut Profile, l: &StoreLine) -> std::result::Result<(String, String, Value), String> {
    let (mod_id, option) = l.key[PREFIX.len()..]
        .split_once('/')
        .filter(|(m, o)| !m.is_empty() && !o.is_empty() && !o.contains('/'))
        .ok_or("not option/<mod id>/<option id>")?;
    let m = manifests.get(mod_id).ok_or("the mod is not in the library")?;
    let value = json_value(l).ok_or("not a value of the type written")?;
    set_option_of(m, profile, option, Some(&value)).map_err(|e| e.to_string())?;
    let stored = profile.options.get(mod_id).and_then(|v| v.get(option)).cloned().unwrap_or(value);
    Ok((mod_id.to_string(), option.to_string(), stored))
}

/// Takes the option values the player changed in the game into the active profile of `data`, the
/// way an edit in the mod details does, writes settings.txt, and removes them from the storage file.
/// Every other line of the file stays as it was, values that don't fit included. Nothing happens
/// while `game_running` says yes, since the game holds the file then; it is asked only when the file
/// has option values, so no file costs one failed read.
pub fn take(data: &Path, saves_dir: &Path, game_running: impl FnOnce() -> bool) -> Result<Taken> {
    let path = store_path(saves_dir);
    let tmp = with_suffix(&path, ".tmp");
    // The loader writes the new file as `.tmp`, deletes the old one and renames. A crash between
    // the two leaves only the `.tmp`, which the loader then reads.
    let from_tmp = !path.is_file();
    let bytes = match std::fs::read(if from_tmp { &tmp } else { &path }) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Taken::default()),
        Err(e) => return Err(e.into()),
    };
    let lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    let pending: Vec<(usize, StoreLine)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| Some((i, parse_line(std::str::from_utf8(l).ok()?)?)))
        .filter(|(_, l)| l.key.starts_with(PREFIX))
        .collect();
    if pending.is_empty() || game_running() {
        return Ok(Taken::default());
    }

    let lib = Library::open(data)?;
    let installed = lib.list()?;
    let mut profile = Profile::load(data)?;
    profile.sync_with(&lib, &installed)?;
    let manifests: BTreeMap<String, Manifest> = installed.into_iter().map(|m| (m.id.clone(), m)).collect();
    let mut done = Taken::default();
    let mut gone = vec![false; lines.len()];
    for (i, l) in &pending {
        match take_one(&manifests, &mut profile, l) {
            Ok(t) => {
                gone[*i] = true;
                done.taken.push(t);
            }
            Err(why) => done.skipped.push((l.key.clone(), l.value.clone(), why)),
        }
    }
    if !done.taken.is_empty() {
        profile.save(data)?;
        write_settings(&lib, &profile, saves_dir)?;
        // The profile holds the values now. Should what follows fail, the next take sets the same
        // values again.
        if from_tmp {
            rename_retrying(&tmp, &path)?;
        }
        let kept: Vec<&[u8]> = lines.iter().zip(&gone).filter(|(_, g)| !**g).map(|(l, _)| *l).collect();
        write_through(&path, &tmp, &kept.join(&b'\n'))?;
    }
    write_log(&path.with_file_name(LOG), &done);
    Ok(done)
}

/// Adds what a take did to the log, which keeps the last `LOG_LINES` lines. A log that can't be
/// written is no reason to stop.
fn write_log(path: &Path, done: &Taken) {
    if done.taken.is_empty() && done.skipped.is_empty() {
        return;
    }
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut lines: Vec<String> = std::fs::read_to_string(path).map(|t| t.lines().map(str::to_string).collect()).unwrap_or_default();
    lines.extend(done.taken.iter().map(|(m, o, v)| format!("{stamp} took {m} {o} = {v}")));
    lines.extend(done.skipped.iter().map(|(k, v, why)| format!("{stamp} skipped {k:?} = {v:?}: {why}")));
    let start = lines.len().saturating_sub(LOG_LINES);
    let _ = std::fs::write(path, lines[start..].join("\n") + "\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::put;
    use serde_json::json;

    struct World {
        _t: tempfile::TempDir,
        data: PathBuf,
        saves: PathBuf,
    }

    impl World {
        fn store(&self) -> PathBuf {
            store_path(&self.saves)
        }
        fn settings(&self) -> Option<String> {
            std::fs::read_to_string(game_side_dir(&self.saves).join(crate::settings::FILE)).ok()
        }
        fn profile(&self) -> Profile {
            Profile::load(&self.data).unwrap()
        }
    }

    /// A library with one mod switched on that has a toggle, a choice and two numbers.
    fn world() -> World {
        let t = crate::testutil::spaced_tempdir();
        let d = t.path().join("stage");
        put(&d, "obsidian.json", br##"{"obsidian":1,"id":"a.opt","version":"1.0.0","name":{"en":"o"},"entry":"main.lua","scope":"client",
            "options":[{"id":"fast","type":"toggle","default":false},{"id":"sky","type":"choice","default":"day","choices":["day","night"]},
                {"id":"speed","type":"number","default":1,"min":1,"max":10,"step":0.5},{"id":"count","type":"number","default":1}]}"##);
        put(&d, "main.lua", b"return true\n");
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        lib.install(&d).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        p.set_enabled("a.opt", true).unwrap();
        p.save(&data).unwrap();
        let saves = t.path().join("Arrowhead").join("Helldivers").join("saves");
        World { _t: t, data, saves }
    }

    fn put_store(w: &World, name: &str, bytes: &[u8]) {
        put(w.store().parent().unwrap(), name, bytes);
    }

    #[test]
    fn lines_unescape_as_the_loader_does() {
        let l = parse_line("a\\tb\\\\c\ts\tx\\ny\\\\z\\q\\").unwrap();
        assert_eq!(l, StoreLine { key: "a\tb\\c".into(), kind: 's', value: "x\ny\\z\\q\\".into() });
        assert_eq!(parse_line("k\ts\ta\tb").unwrap().value, "a\tb", "the value runs to the end of the line");
        assert_eq!(parse_line("k\\r\\n\tb\t\\1").unwrap(), StoreLine { key: "k\r\n".into(), kind: 'b', value: "\\1".into() }, "only strings unescape");
        assert_eq!(parse_line("k\tn\t2.5").unwrap().value, "2.5");
        for bad in ["no tabs", "k\tx\t1", "k\tss\t1", "k\ts"] {
            assert_eq!(parse_line(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_toggle_a_choice_and_numbers_are_taken() {
        let w = world();
        put_store(&w, STORE, b"api/seen\tb\t1\noption/a.opt/fast\tb\t1\noption/a.opt/sky\ts\tnight\noption/a.opt/speed\tn\t2.5\noption/a.opt/count\tn\t4\nzeta\ts\tkept\n");
        let done = take(&w.data, &w.saves, || false).unwrap();
        assert_eq!(done.taken.len(), 4);
        assert!(done.skipped.is_empty());

        let values = &w.profile().options["a.opt"];
        assert_eq!((&values["fast"], &values["sky"], &values["speed"], &values["count"]), (&json!(true), &json!("night"), &json!(2.5), &json!(4)));
        assert!(values["count"].is_i64(), "a whole number stays whole");
        let s = w.settings().expect("settings.txt written");
        for line in ["option\ta.opt\tfast\ttoggle\t1\n", "option\ta.opt\tsky\tchoice\tnight\n", "option\ta.opt\tspeed\tnumber\t2.5\n", "option\ta.opt\tcount\tnumber\t4\n"] {
            assert!(s.contains(line), "{line:?} in\n{s}");
        }
        assert_eq!(std::fs::read(w.store()).unwrap(), b"api/seen\tb\t1\nzeta\ts\tkept\n");
        assert!(!with_suffix(&w.store(), ".tmp").exists());
        let log = std::fs::read_to_string(w.store().with_file_name(LOG)).unwrap();
        assert!(log.contains("took a.opt sky = \"night\""), "{log}");

        assert_eq!(take(&w.data, &w.saves, || false).unwrap(), Taken::default(), "nothing left to take");
    }

    #[test]
    fn values_that_dont_fit_are_skipped_and_kept() {
        let w = world();
        let text: &[u8] = b"option/a.opt/speed\tn\t11\noption/a.opt/speed\tn\t2.25\noption/a.opt/count\tn\tnan\noption/a.opt/count\tn\t1.#INF\n\
            option/a.opt/sky\ts\tnoon\noption/a.opt/sky\tb\t1\noption/a.opt/fast\ts\t1\noption/a.opt/fast\tb\t2\n\
            option/a.opt/nope\tb\t1\noption/gone.mod/fast\tb\t1\noption/a.opt\tb\t1\noption/a.opt/fast/x\tb\t1\n";
        put_store(&w, STORE, text);
        let done = take(&w.data, &w.saves, || false).unwrap();
        assert!(done.taken.is_empty(), "{:?}", done.taken);
        assert_eq!(done.skipped.len(), 12);
        assert!(w.profile().options.is_empty());
        assert_eq!(w.settings(), None, "nothing taken, nothing written");
        assert_eq!(std::fs::read(w.store()).unwrap(), text);
        let log = std::fs::read_to_string(w.store().with_file_name(LOG)).unwrap();
        assert!(log.contains("skipped \"option/gone.mod/fast\" = \"1\": the mod is not in the library"), "{log}");
        assert!(log.contains("mod a.opt has no option nope"), "{log}");
    }

    #[test]
    fn other_lines_are_kept_byte_for_byte() {
        let w = world();
        let before: &[&[u8]] = &[
            b"zeta\ts\tkept \\t\\n\\\\ x\r\n",
            b"\n",
            b"no tabs here\n",
            b"\xff\xfe not text\toption/x\n",
            b"option/a.opt/fast\tb\t1\n",
            b"api/option/a.opt/sky\ts\tnight\n",
            b"option/a.opt/sky\ts\tnight\r\n",
            b"last\ts\tno line break",
        ];
        put_store(&w, STORE, &before.concat());
        let done = take(&w.data, &w.saves, || false).unwrap();
        assert_eq!(done.taken, [("a.opt".to_string(), "fast".to_string(), json!(true))]);
        assert_eq!(done.skipped.len(), 1, "night with a carriage return isn't a choice");
        let after: Vec<&[u8]> = before.iter().copied().filter(|l| *l != b"option/a.opt/fast\tb\t1\n").collect();
        assert_eq!(std::fs::read(w.store()).unwrap(), after.concat());
    }

    #[test]
    fn the_tmp_file_is_read_when_the_store_is_missing() {
        let w = world();
        put_store(&w, &format!("{STORE}.tmp"), b"keep\tn\t1\noption/a.opt/sky\ts\tnight\n");
        let done = take(&w.data, &w.saves, || false).unwrap();
        assert_eq!(done.taken.len(), 1);
        assert_eq!(w.profile().options["a.opt"]["sky"], json!("night"));
        assert_eq!(std::fs::read(w.store()).unwrap(), b"keep\tn\t1\n");
        assert!(!with_suffix(&w.store(), ".tmp").exists(), "the .tmp became the store");
    }

    #[test]
    fn nothing_is_done_while_the_game_runs() {
        let w = world();
        assert_eq!(take(&w.data, &w.saves, || panic!("no file, so no need to ask")).unwrap(), Taken::default());
        put_store(&w, STORE, b"other\ts\tx\n");
        assert_eq!(take(&w.data, &w.saves, || panic!("no option values, so no need to ask")).unwrap(), Taken::default());

        let text: &[u8] = b"other\ts\tx\noption/a.opt/fast\tb\t1\n";
        put_store(&w, STORE, text);
        assert_eq!(take(&w.data, &w.saves, || true).unwrap(), Taken::default());
        assert_eq!(std::fs::read(w.store()).unwrap(), text);
        assert!(w.profile().options.is_empty());
        assert_eq!(w.settings(), None);
        assert!(!w.store().with_file_name(LOG).exists());
    }
}
