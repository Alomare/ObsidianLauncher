//! `obsidian.json`: what a mod is and which game resources it changes.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hash::parse_ref;
use crate::{ObsidianError, Result};

pub const FILE: &str = "obsidian.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    Client,
    /// Anything that may change the shared game. Missing or unknown scopes count as session.
    #[default]
    Session,
    /// Code for other mods: session when an enabled session mod requires it, client otherwise.
    Library,
}

impl Serialize for Scope {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Scope::Client => "client",
            Scope::Session => "session",
            Scope::Library => "library",
        })
    }
}

impl<'de> Deserialize<'de> for Scope {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Ok(match v.as_str() {
            Some("client") => Scope::Client,
            Some("library") => Scope::Library,
            _ => Scope::Session,
        })
    }
}

pub type Localized = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    Source(PathBuf),
    Bytecode(PathBuf),
    Data(PathBuf),
    /// Raw payloads per language, each with the part the game reads from the bundle's `.stream`: audio
    /// banks and videos keep most of their bytes there.
    Variants(Vec<VariantSpec>),
    /// A sound bank rebuilt from the installed game at deploy, with these clips in place of its own.
    Clips(Vec<ClipSpec>),
}

/// One clip of a sound bank in one language: `clip` is `#` and 16 hex digits, `data` the bytes in the bank
/// (a 68-byte header and the first part of the audio), `stream` the bytes in the stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipSpec {
    pub clip: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub lang: u32,
    pub data: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<PathBuf>,
}

impl ClipSpec {
    /// The clip's id. `ResourceSpec` has checked the text.
    pub fn id(&self) -> u64 {
        u64::from_str_radix(self.clip.trim_start_matches('#'), 16).unwrap_or(0)
    }
}

/// One language of a resource: `lang` is the engine's language flag (0 for every language without its
/// own variant, 0x100 French, 0x4000 German...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantSpec {
    #[serde(default)]
    pub lang: u32,
    pub data: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<PathBuf>,
}

impl Payload {
    /// The main file: the payload, or the first variant's data.
    pub fn path(&self) -> &Path {
        match self {
            Payload::Source(p) | Payload::Bytecode(p) | Payload::Data(p) => p,
            Payload::Variants(v) => &v[0].data,
            Payload::Clips(c) => &c[0].data,
        }
    }

    /// Every file the payload reads.
    pub fn paths(&self) -> Vec<&Path> {
        match self {
            Payload::Variants(v) => v.iter().flat_map(|v| std::iter::once(v.data.as_path()).chain(v.stream.as_deref())).collect(),
            Payload::Clips(c) => c.iter().flat_map(|c| std::iter::once(c.data.as_path()).chain(c.stream.as_deref())).collect(),
            _ => vec![self.path()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawResource", into = "RawResource")]
pub struct ResourceSpec {
    pub type_text: String,
    pub name_text: String,
    pub type_hash: u64,
    pub name_hash: u64,
    pub bundle: Option<String>,
    pub flags: u32,
    pub payload: Payload,
    /// Deploy options: the resource joins only when every listed option has the listed value.
    pub when: BTreeMap<String, Value>,
}

impl ResourceSpec {
    pub fn data_path(&self) -> &Path {
        self.payload.path()
    }
}

#[derive(Serialize, Deserialize)]
struct RawResource {
    #[serde(rename = "type")]
    type_: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bundle: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    flags: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bytecode: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<PathBuf>,
    /// With `data`: the part of the resource that goes into the patch's `.stream`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    variants: Vec<VariantSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clips: Vec<ClipSpec>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    when: BTreeMap<String, Value>,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl TryFrom<RawResource> for ResourceSpec {
    type Error = String;
    fn try_from(r: RawResource) -> std::result::Result<Self, String> {
        let clips = !r.clips.is_empty();
        if clips {
            if r.type_ != "timpani_bank" {
                return Err("clips goes only with the type timpani_bank".into());
            }
            if r.bundle.is_some() || r.flags != 0 {
                return Err("clips leaves out bundle and flags: Obsidian finds the bundles that hold the bank".into());
            }
            if r.source.is_some() || r.bytecode.is_some() || r.data.is_some() || r.stream.is_some() || !r.variants.is_empty() {
                return Err("needs exactly one of source, bytecode, data, variants or clips".into());
            }
            let mut seen: Vec<(u32, &str)> = Vec::new();
            for c in &r.clips {
                if !c.clip.strip_prefix('#').is_some_and(is_hex16) {
                    return Err(format!("clip {} is not # and 16 lowercase hex digits", c.clip));
                }
                if seen.contains(&(c.lang, c.clip.as_str())) {
                    return Err(format!("clip {} is listed twice for lang {}", c.clip, c.lang));
                }
                seen.push((c.lang, c.clip.as_str()));
            }
        }
        let payload = match (r.source, r.bytecode, r.data, r.variants.is_empty()) {
            _ if clips => Payload::Clips(r.clips),
            (Some(p), None, None, true) if r.stream.is_none() => Payload::Source(p),
            (None, Some(p), None, true) if r.stream.is_none() => Payload::Bytecode(p),
            (None, None, Some(p), true) => match r.stream {
                None => Payload::Data(p),
                stream => Payload::Variants(vec![VariantSpec { lang: 0, data: p, stream }]),
            },
            (None, None, None, false) if r.stream.is_none() => {
                let mut langs: Vec<u32> = r.variants.iter().map(|v| v.lang).collect();
                langs.sort_unstable();
                if langs.windows(2).any(|w| w[0] == w[1]) {
                    return Err("two variants have the same lang".into());
                }
                Payload::Variants(r.variants)
            }
            _ => return Err("needs exactly one of source, bytecode, data, variants or clips (stream goes only with data)".into()),
        };
        Ok(ResourceSpec {
            type_hash: parse_ref(&r.type_).map_err(|e| e.to_string())?,
            name_hash: parse_ref(&r.name).map_err(|e| e.to_string())?,
            type_text: r.type_,
            name_text: r.name,
            bundle: r.bundle,
            flags: r.flags,
            payload,
            when: r.when,
        })
    }
}

impl From<ResourceSpec> for RawResource {
    fn from(s: ResourceSpec) -> Self {
        let (mut source, mut bytecode, mut data, mut stream, mut variants, mut clips) = (None, None, None, None, Vec::new(), Vec::new());
        match s.payload {
            Payload::Source(p) => source = Some(p),
            Payload::Bytecode(p) => bytecode = Some(p),
            Payload::Data(p) => data = Some(p),
            // One variant for every language is written the short way, as an author would.
            Payload::Variants(v) if v.len() == 1 && v[0].lang == 0 => {
                let v = v.into_iter().next().unwrap();
                (data, stream) = (Some(v.data), v.stream);
            }
            Payload::Variants(v) => variants = v,
            Payload::Clips(c) => clips = c,
        }
        RawResource { type_: s.type_text, name: s.name_text, bundle: s.bundle, flags: s.flags, source, bytecode, data, stream, variants, clips, when: s.when }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub obsidian: u32,
    pub id: String,
    pub version: semver::Version,
    pub name: Localized,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Localized>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub scope: Scope,
    /// Runs only while every other player in the lobby runs it too, at the same version and with
    /// the same shared option values. Players who allow other players' mods don't count.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub every_player: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<PathBuf>,
    /// More Lua scripts beside `entry`, which the mod loads with `obsidian.module`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<PathBuf>,
    #[serde(default)]
    pub resources: Vec<ResourceSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keybinds: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    #[serde(default = "one")]
    pub loader_api: u32,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn one() -> u32 {
    1
}

fn bad(field: impl Into<String>, reason: impl Into<String>) -> ObsidianError {
    ObsidianError::BadManifest { field: field.into(), reason: reason.into() }
}

fn is_hex16(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(c))
}

/// A relative path that stays inside the mod folder and names an existing file.
fn check_path(mod_dir: &Path, p: &Path, field: &str) -> Result<()> {
    if !p.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(bad(field, format!("{} must be a path inside the mod folder", p.display())));
    }
    if !mod_dir.join(p).is_file() {
        return Err(bad(field, format!("{} does not exist", p.display())));
    }
    Ok(())
}

impl Manifest {
    /// Ships with Obsidian: always in the library, can't be removed.
    pub fn validate(&self, mod_dir: &Path) -> Result<()> {
        if self.obsidian != 1 {
            return Err(bad("obsidian", "only version 1 is supported"));
        }
        if !valid_id(&self.id) {
            return Err(bad("id", "use 1 to 64 of a-z, 0-9, '.', '_' or '-', starting with a letter or digit"));
        }
        if !self.name.contains_key("en") {
            return Err(bad("name.en", "an English name is required"));
        }
        if self.every_player && self.scope == Scope::Client {
            return Err(bad("every_player", "a client mod reaches no other player, so use session or library"));
        }
        if let Some(entry) = &self.entry {
            check_path(mod_dir, entry, "entry")?;
        }
        for (i, p) in self.modules.iter().enumerate() {
            let field = format!("modules[{i}]");
            check_path(mod_dir, p, &field)?;
            if p.extension().is_none_or(|x| x != "lua") {
                return Err(bad(field, "a module is a .lua file"));
            }
            if p.with_extension("") == Path::new("main") {
                return Err(bad(field, "main is the name the entry script runs under"));
            }
        }
        for id in &self.requires {
            if !valid_id(id) || *id == self.id {
                return Err(bad("requires", format!("{id} is not the id of another mod")));
            }
        }
        let options = self.option_specs()?;
        crate::options::parse_keybinds(&self.keybinds)?;
        for (i, r) in self.resources.iter().enumerate() {
            let field = format!("resources[{i}]");
            if r.bundle.as_deref().is_some_and(|b| !is_hex16(b)) {
                return Err(bad(field, "bundle must be 16 lowercase hex digits"));
            }
            for p in r.payload.paths() {
                check_path(mod_dir, p, &field)?;
            }
            for (id, value) in &r.when {
                let spec = options.iter().find(|o| o.id == *id).ok_or_else(|| bad(&field, format!("when names {id}, which is not an option")))?;
                crate::options::validate(spec, value).map_err(|_| bad(&field, format!("when: {value} is not a value of {id}")))?;
            }
        }
        Ok(())
    }

    pub fn option_specs(&self) -> Result<Vec<crate::options::OptionSpec>> {
        crate::options::parse_options(&self.options)
    }
}

/// When a manifest last changed and the file size: a cached parse stays good while this is the same.
/// Checking a manifest reads no other file, so the manifest alone decides.
type Stamp = (std::time::SystemTime, u64);

fn stamp(mod_dir: &Path) -> Option<Stamp> {
    let f = std::fs::metadata(mod_dir.join(FILE)).ok()?;
    Some((f.modified().ok()?, f.len()))
}

/// Parsed manifests by mod folder. Every action lists the library, and parsing and checking
/// hundreds of manifests each time took most of the time an action waits.
fn cache() -> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, (Stamp, Manifest)>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, (Stamp, Manifest)>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

pub fn load_manifest(mod_dir: &Path) -> Result<Manifest> {
    let stamp = stamp(mod_dir);
    if let Some(s) = stamp
        && let Some((cached, m)) = cache().lock().unwrap().get(mod_dir)
        && *cached == s
    {
        return Ok(m.clone());
    }
    let m = read_manifest(mod_dir)?;
    if let Some(s) = stamp {
        cache().lock().unwrap().insert(mod_dir.to_path_buf(), (s, m.clone()));
    }
    Ok(m)
}

fn read_manifest(mod_dir: &Path) -> Result<Manifest> {
    let text = std::fs::read_to_string(mod_dir.join(FILE))?;
    let mut v: Value = serde_json::from_str(&text).map_err(|e| bad(FILE, e.to_string()))?;
    let raw = v.as_object_mut().ok_or_else(|| bad(FILE, "must be a JSON object"))?.remove("resources");
    let resources = match raw {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .into_iter()
            .enumerate()
            .map(|(i, r)| serde_json::from_value(r).map_err(|e| bad(format!("resources[{i}]"), e.to_string())))
            .collect::<Result<Vec<ResourceSpec>>>()?,
        Some(_) => return Err(bad("resources", "must be a list")),
    };
    let mut m: Manifest = serde_json::from_value(v).map_err(|e| bad(FILE, e.to_string()))?;
    m.resources = resources;
    m.validate(mod_dir)?;
    Ok(m)
}

/// Drops the cached parse of a mod folder's manifest. The library calls it whenever it writes
/// or removes one: the clock can give two writes in quick succession the same time.
pub fn forget(mod_dir: &Path) {
    cache().lock().unwrap().remove(mod_dir);
}

pub fn write_manifest(mod_dir: &Path, m: &Manifest) -> Result<()> {
    forget(mod_dir);
    std::fs::write(mod_dir.join(FILE), serde_json::to_string_pretty(m)? + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    const EXAMPLE: &str = r#"{
      "obsidian": 1, "id": "author.short-name", "version": "1.0.0",
      "name": { "en": "Example", "pt-BR": "Exemplo" }, "description": { "en": "x" },
      "authors": ["Author"], "scope": "client", "entry": "src/main.lua",
      "resources": [
        { "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" },
        { "type": "texture", "name": "units/foo/bar", "bundle": "ebbaccd983625dde", "data": "res/bar.texture" }
      ],
      "options": [ { "id": "distance", "type": "number", "default": 5 } ],
      "keybinds": [ { "id": "toggle", "default": "f8" } ],
      "requires": [], "conflicts": [], "loader_api": 1, "homepage": "https://example.org"
    }"#;

    fn mod_with(json: &str) -> tempfile::TempDir {
        let d = spaced_tempdir();
        put(d.path(), FILE, json.as_bytes());
        for f in ["src/main.lua", "res/perk_settings.lua", "res/bar.texture"] {
            put(d.path(), f, b"x");
        }
        d
    }

    fn field_of(r: Result<Manifest>) -> String {
        match r {
            Err(ObsidianError::BadManifest { field, .. }) => field,
            other => panic!("expected BadManifest, got {other:?}"),
        }
    }

    #[test]
    fn spec_example_parses() {
        let m = load_manifest(mod_with(EXAMPLE).path()).unwrap();
        assert_eq!(m.scope, Scope::Client);
        assert_eq!(m.resources.len(), 2);
        assert_eq!(m.resources[0].type_hash, 0xa14e8dfa2cd117e2);
    }

    #[test]
    fn unknown_or_missing_scope_is_session() {
        let weird = EXAMPLE.replace(r#""scope": "client""#, r#""scope": "weird""#);
        assert_eq!(load_manifest(mod_with(&weird).path()).unwrap().scope, Scope::Session);
        let missing = EXAMPLE.replace(r#""scope": "client","#, "");
        assert_eq!(load_manifest(mod_with(&missing).path()).unwrap().scope, Scope::Session);
        let library = EXAMPLE.replace(r#""scope": "client""#, r#""scope": "library""#);
        assert_eq!(load_manifest(mod_with(&library).path()).unwrap().scope, Scope::Library);
    }

    #[test]
    fn two_payloads_are_rejected() {
        let both = EXAMPLE.replace(r#""source": "res/perk_settings.lua""#, r#""source": "res/perk_settings.lua", "data": "res/bar.texture""#);
        assert_eq!(field_of(load_manifest(mod_with(&both).path())), "resources[0]");
    }

    #[test]
    fn every_player_needs_a_mod_that_reaches_other_players() {
        assert!(!load_manifest(mod_with(EXAMPLE).path()).unwrap().every_player);
        let client = EXAMPLE.replace(r#""scope": "client","#, r#""scope": "client", "every_player": true,"#);
        assert_eq!(field_of(load_manifest(mod_with(&client).path())), "every_player");
        for scope in ["session", "library"] {
            let ok = EXAMPLE.replace(r#""scope": "client","#, &format!(r#""scope": "{scope}", "every_player": true,"#));
            let d = mod_with(&ok);
            let m = load_manifest(d.path()).unwrap();
            assert!(m.every_player, "{scope}");
            write_manifest(d.path(), &m).unwrap();
            assert_eq!(load_manifest(d.path()).unwrap(), m, "it survives a round trip");
        }
    }

    #[test]
    fn english_name_is_required() {
        let no_en = EXAMPLE.replace(r#""en": "Example", "#, "");
        assert_eq!(field_of(load_manifest(mod_with(&no_en).path())), "name.en");
    }

    #[test]
    fn payload_outside_the_mod_is_rejected() {
        let out = EXAMPLE.replace("res/bar.texture", "../x");
        assert_eq!(field_of(load_manifest(mod_with(&out).path())), "resources[1]");
    }

    #[test]
    fn options_and_keybinds_are_checked() {
        let bad_default = EXAMPLE.replace(r#""default": 5"#, r#""default": "far""#);
        assert_eq!(field_of(load_manifest(mod_with(&bad_default).path())), "options[0]");
        let bad_key = EXAMPLE.replace(r#""default": "f8""#, r#""default": "F 8""#);
        assert_eq!(field_of(load_manifest(mod_with(&bad_key).path())), "keybinds[0]");
    }

    #[test]
    fn when_needs_a_declared_option_and_a_valid_value() {
        let unknown = EXAMPLE.replace(r#""data": "res/bar.texture""#, r#""data": "res/bar.texture", "when": {"sky": "day"}"#);
        assert_eq!(field_of(load_manifest(mod_with(&unknown).path())), "resources[1]");
        let invalid = EXAMPLE.replace(r#""data": "res/bar.texture""#, r#""data": "res/bar.texture", "when": {"distance": "far"}"#);
        assert_eq!(field_of(load_manifest(mod_with(&invalid).path())), "resources[1]");
        let good = EXAMPLE.replace(r#""data": "res/bar.texture""#, r#""data": "res/bar.texture", "when": {"distance": 5}"#);
        assert!(load_manifest(mod_with(&good).path()).is_ok());
    }

    #[test]
    fn unknown_fields_survive_a_round_trip() {
        let d = mod_with(EXAMPLE);
        let m = load_manifest(d.path()).unwrap();
        write_manifest(d.path(), &m).unwrap();
        let again = load_manifest(d.path()).unwrap();
        assert_eq!(again.extra.get("homepage").and_then(Value::as_str), Some("https://example.org"));
        assert_eq!(again, m);
    }

    const BANK: &str = r##"{ "type": "timpani_bank", "name": "#e709d0c5697fb286", "bundle": "0e2ec9da60a0f53a", "##;

    #[test]
    fn a_recipe_lists_clips_and_nothing_else_on_the_resource() {
        let good = r##"{"type":"timpani_bank","name":"#e0f28740d6c19769","clips":[
            {"clip":"#1111111111111111","data":"res/bank.bin","stream":"res/bank.stream"},
            {"clip":"#1111111111111111","lang":256,"data":"res/bank.fr.bin"}]}"##;
        let d = audio_mod(good);
        let m = load_manifest(d.path()).unwrap();
        let Payload::Clips(c) = &m.resources[0].payload else { panic!() };
        assert_eq!((c[0].id(), c[0].lang, c[1].lang, c[1].stream.is_none()), (0x1111111111111111, 0, 256, true));
        assert_eq!(m.resources[0].payload.paths().len(), 3);
        let again: ResourceSpec = serde_json::from_value(serde_json::to_value(&m.resources[0]).unwrap()).unwrap();
        assert_eq!(again, m.resources[0], "it survives a round trip");
        for (bad, why) in [
            (r##"{"type":"lua","name":"#00000000000000f1","clips":[{"clip":"#1111111111111111","data":"res/bank.bin"}]}"##, "only with"),
            (r##"{"type":"timpani_bank","name":"#00000000000000f1","bundle":"00000000000000aa","clips":[{"clip":"#1111111111111111","data":"res/bank.bin"}]}"##, "leaves out bundle"),
            (r##"{"type":"timpani_bank","name":"#00000000000000f1","data":"res/bank.bin","clips":[{"clip":"#1111111111111111","data":"res/bank.bin"}]}"##, "exactly one"),
            (r##"{"type":"timpani_bank","name":"#00000000000000f1","clips":[{"clip":"1111111111111111","data":"res/bank.bin"}]}"##, "16 lowercase"),
            (r##"{"type":"timpani_bank","name":"#00000000000000f1","clips":[{"clip":"#1111111111111111","data":"res/bank.bin"},{"clip":"#1111111111111111","data":"res/bank.fr.bin"}]}"##, "twice"),
        ] {
            let d = audio_mod(bad);
            let e = load_manifest(d.path()).unwrap_err().to_string();
            assert!(e.contains(why), "{bad}: {e}");
        }
    }

    fn audio_mod(resource: &str) -> tempfile::TempDir {
        let d = mod_with(&EXAMPLE.replace(r#"{ "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" },"#, &format!("{resource},")));
        for f in ["res/bank.bin", "res/bank.stream", "res/bank.fr.bin", "res/bank.fr.stream"] {
            put(d.path(), f, b"x");
        }
        d
    }

    #[test]
    fn a_bank_with_a_stream_or_language_variants_parses_and_round_trips() {
        let d = audio_mod(&format!(r#"{BANK} "data": "res/bank.bin", "stream": "res/bank.stream" }}"#));
        let m = load_manifest(d.path()).unwrap();
        let want = vec![VariantSpec { lang: 0, data: "res/bank.bin".into(), stream: Some("res/bank.stream".into()) }];
        assert_eq!(m.resources[0].payload, Payload::Variants(want));
        write_manifest(d.path(), &m).unwrap();
        let text = std::fs::read_to_string(d.path().join(FILE)).unwrap();
        assert!(text.contains(r#""stream": "res/bank.stream""#) && !text.contains("variants"), "one variant is written the short way");

        let d = audio_mod(&format!(
            r#"{BANK} "variants": [ {{ "data": "res/bank.bin", "stream": "res/bank.stream" }}, {{ "lang": 256, "data": "res/bank.fr.bin", "stream": "res/bank.fr.stream" }} ] }}"#
        ));
        let m = load_manifest(d.path()).unwrap();
        let Payload::Variants(v) = &m.resources[0].payload else { panic!() };
        assert_eq!(v.iter().map(|v| v.lang).collect::<Vec<_>>(), [0, 256]);
        assert_eq!(m.resources[0].payload.paths().len(), 4);
        write_manifest(d.path(), &m).unwrap();
        assert_eq!(load_manifest(d.path()).unwrap(), m);
    }

    #[test]
    fn variants_are_checked() {
        let twice = format!(r#"{BANK} "variants": [ {{ "data": "res/bank.bin" }}, {{ "lang": 0, "data": "res/bank.fr.bin" }} ] }}"#);
        assert_eq!(field_of(load_manifest(audio_mod(&twice).path())), "resources[0]");
        let both = format!(r#"{BANK} "data": "res/bank.bin", "variants": [ {{ "data": "res/bank.bin" }} ] }}"#);
        assert_eq!(field_of(load_manifest(audio_mod(&both).path())), "resources[0]");
        let lua_stream = format!(r#"{BANK} "source": "res/bank.bin", "stream": "res/bank.stream" }}"#);
        assert_eq!(field_of(load_manifest(audio_mod(&lua_stream).path())), "resources[0]");
        let gone = format!(r#"{BANK} "data": "res/bank.bin", "stream": "res/missing.stream" }}"#);
        assert_eq!(field_of(load_manifest(audio_mod(&gone).path())), "resources[0]");
    }
}
