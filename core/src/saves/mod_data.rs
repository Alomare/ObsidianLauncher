//! Per-save mod data: the text file beside a save, `<mod id>\t<key>\t<s|b|n>\t<value>` per line,
//! with the escapes of the loader's storage.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModValue {
    Text(String),
    Toggle(bool),
    Number(f64),
}

impl ModValue {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModData(pub BTreeMap<String, BTreeMap<String, ModValue>>);

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r")
}

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

/// A number written so the loader reads the same value back.
fn number_text(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 { format!("{n:.0}") } else { n.to_string() }
}

/// Lines that don't parse are skipped, as the loader does.
pub fn parse(text: &str) -> ModData {
    let mut data = ModData::default();
    for line in text.lines() {
        let mut f = line.splitn(4, '\t');
        let (Some(id), Some(key), Some(kind), Some(value)) = (f.next(), f.next(), f.next(), f.next()) else { continue };
        let value = match kind {
            "s" => ModValue::Text(unescape(value)),
            "b" => ModValue::Toggle(value == "1"),
            "n" => match value.parse() {
                Ok(n) => ModValue::Number(n),
                Err(_) => continue,
            },
            _ => continue,
        };
        data.0.entry(id.to_string()).or_default().insert(unescape(key), value);
    }
    data
}

pub fn render(data: &ModData) -> String {
    let mut out = String::new();
    for (id, values) in &data.0 {
        for (key, value) in values {
            let (kind, text) = match value {
                ModValue::Text(s) => ("s", escape(s)),
                ModValue::Toggle(b) => ("b", (if *b { "1" } else { "0" }).to_string()),
                ModValue::Number(n) => ("n", number_text(*n)),
            };
            out.push_str(&format!("{id}\t{}\t{kind}\t{text}\n", escape(key)));
        }
    }
    out
}

pub fn read(path: &Path) -> Result<ModData> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(parse(&t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ModData::default()),
        Err(e) => Err(e.into()),
    }
}

/// Writes atomically, or removes the file when nothing is left.
pub fn write(path: &Path, data: &ModData) -> Result<()> {
    let empty = data.0.values().all(BTreeMap::is_empty);
    if empty {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    } else {
        crate::paths::write_atomic(path, render(data).as_bytes()).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_with_escapes() {
        let mut d = ModData::default();
        let m = d.0.entry("a.b".into()).or_default();
        m.insert("k\twith\ttabs".into(), ModValue::Text("line\nbreak\\end".into()));
        m.insert("flag".into(), ModValue::Toggle(true));
        m.insert("n".into(), ModValue::Number(2.5));
        m.insert("whole".into(), ModValue::Number(7.0));
        let text = render(&d);
        assert!(text.contains("a.b\tflag\tb\t1\n"));
        assert!(text.contains("a.b\twhole\tn\t7\n"));
        assert_eq!(parse(&text), d);
        assert_eq!(parse("bad line\na.b\tk\tq\tv\nx.y\tz\ts\tok\n").0.len(), 1);
    }

    #[test]
    fn an_empty_set_removes_the_file() {
        let t = crate::testutil::spaced_tempdir();
        let p = t.path().join("x.obsidian");
        let mut d = ModData::default();
        d.0.entry("a.b".into()).or_default().insert("k".into(), ModValue::Toggle(false));
        write(&p, &d).unwrap();
        assert!(p.is_file());
        assert_eq!(read(&p).unwrap(), d);
        write(&p, &ModData::default()).unwrap();
        assert!(!p.exists());
        assert_eq!(read(&p).unwrap(), ModData::default());
    }
}
