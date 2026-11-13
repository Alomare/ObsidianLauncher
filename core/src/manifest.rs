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
    /// Anything that may change the shared game.
    Session,
    /// Code for other mods: session when an enabled session mod requires it, client otherwise.
    Library,
    /// Content of its own, such as a game mode, that leaves the rest of the game alone. A client mod
    /// at deploy, with its hooks off, until the host starts the content and every player runs it.
    Expansion,
    /// No scope given, or one Obsidian doesn't know, as with mods imported from elsewhere. It runs
    /// as a client mod: being kept out of multiplayer by a guess is worse than what it might change.
    #[default]
    Unspecified,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Scope::Client => "client",
            Scope::Session => "session",
            Scope::Library => "library",
            Scope::Expansion => "expansion",
            Scope::Unspecified => "unspecified",
        }
    }

    /// Runs as a client mod: declared one, or declared nothing. An expansion isn't one: its shared
    /// options go into what players compare, so changing one needs a deploy.
    pub fn is_client(self) -> bool {
        matches!(self, Scope::Client | Scope::Unspecified)
    }

    fn is_unspecified(&self) -> bool {
        *self == Scope::Unspecified
    }
}

impl Serialize for Scope {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Scope {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Ok(match v.as_str() {
            Some("client") => Scope::Client,
            Some("session") => Scope::Session,
            Some("library") => Scope::Library,
            Some("expansion") => Scope::Expansion,
            _ => Scope::Unspecified,
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
    /// The secondary scope the file belongs to (`secondary_scope`): left out while that one is off.
    pub part: Option<String>,
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
    /// `part` before Hybrid Scopes had the name.
    #[serde(default, rename = "secondary_scope", alias = "part", skip_serializing_if = "Option::is_none")]
    part: Option<String>,
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
            part: r.part,
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
        RawResource { type_: s.type_text, name: s.name_text, bundle: s.bundle, flags: s.flags, source, bytecode, data, stream, variants, clips, when: s.when, part: s.part }
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
    #[serde(default, skip_serializing_if = "Scope::is_unspecified")]
    pub scope: Scope,
    /// Runs only while every other player in the lobby runs it too, at the same version and with
    /// the same shared option values. Players who allow other players' mods don't count.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub every_player: bool,
    /// Hybrid Scopes: code, options and files of the mod that run with a scope other than the primary
    /// `scope` (parts.rs). The primary scope is all an older Obsidian reads. `parts` before the feature
    /// had that name.
    #[serde(default, rename = "secondary_scopes", alias = "parts", skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<PartSpec>,
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
    /// Where this mod asks to load: `"first"` or `"last"`, or rules with before and after (`LoadRules`).
    /// A new mod takes its place on joining a profile, and Sort Mods puts the whole list in line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load: Option<LoadRules>,
    /// Ids of mods this one asks to load after: the form before `load.after`, read the same way.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub load_after: Vec<String>,
    #[serde(default = "one")]
    pub loader_api: u32,
    /// The oldest Obsidian the mod works with. A newer one than this is refused, asking for an update,
    /// instead of deploying something it can't do right. Obsidian before 1.1 doesn't read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_obsidian: Option<semver::Version>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn one() -> u32 {
    1
}

/// One secondary scope of a mod (`secondary_scopes`): the code, options and files tagged with the id run with this scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartSpec {
    pub id: String,
    /// `client` or `session`; anything else is refused when the manifest is checked.
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub name: Localized,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Localized>,
    /// A toggle option of the mod: the secondary scope is on only while that option is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A mod's items and fields in the save editor (`save_editor` in the manifest).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SaveEditorSpec {
    #[serde(default)]
    pub items: Vec<SaveItemSpec>,
    #[serde(default)]
    pub fields: Vec<SaveFieldSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveItemSpec {
    /// `stratagem`, `weapon` or `perk`.
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub label: Option<Value>,
    /// `manual`, `free`, `base` or `{ "rank": n }`.
    #[serde(default)]
    pub unlock: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFieldSpec {
    pub id: String,
    /// `number`, `toggle`, `choice`, or the reserved `tree`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub label: Option<Value>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub choices: Option<Vec<Value>>,
    #[serde(default)]
    pub when: Option<Value>,
}

impl Manifest {
    /// `save_editor`, parsed. None when the mod has none.
    pub fn save_editor(&self) -> Result<Option<SaveEditorSpec>> {
        let Some(v) = self.extra.get("save_editor") else { return Ok(None) };
        let spec: SaveEditorSpec = serde_json::from_value(v.clone()).map_err(|e| bad("save_editor", e.to_string()))?;
        for i in &spec.items {
            if !["stratagem", "weapon", "perk"].contains(&i.kind.as_str()) {
                return Err(bad("save_editor.items", format!("unknown type {}", i.kind)));
            }
            let unlock_fine = match &i.unlock {
                None => true,
                Some(Value::String(s)) => ["manual", "free", "base"].contains(&s.as_str()),
                Some(Value::Object(o)) => o.get("rank").is_some_and(Value::is_u64),
                _ => false,
            };
            if !unlock_fine {
                return Err(bad("save_editor.items", format!("{} has an unknown unlock", i.name)));
            }
        }
        for f in &spec.fields {
            if !["number", "toggle", "choice", "tree"].contains(&f.kind.as_str()) {
                return Err(bad("save_editor.fields", format!("unknown type {}", f.kind)));
            }
        }
        Ok(Some(spec))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    First,
    Last,
}

/// `load` in a manifest. The text `"first"` or `"last"` is short for `{ "position": ... }`, and Obsidian
/// 1.3.1 and earlier read only `"last"`, so a position alone is written that way.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadRules {
    pub position: Option<Position>,
    pub before: Vec<String>,
    pub after: Vec<String>,
    /// Before every mod that replaces game files (has resources).
    pub before_patches: bool,
    pub after_patches: bool,
}

impl LoadRules {
    fn position_from(text: &str) -> std::result::Result<Position, String> {
        match text {
            "first" => Ok(Position::First),
            "last" => Ok(Position::Last),
            _ => Err(format!("load: position is \"first\" or \"last\", not \"{text}\"")),
        }
    }
}

impl TryFrom<Value> for LoadRules {
    type Error = String;
    fn try_from(v: Value) -> std::result::Result<Self, String> {
        let obj = match v {
            Value::String(s) => return Ok(LoadRules { position: Some(LoadRules::position_from(&s)?), ..Default::default() }),
            Value::Object(o) => o,
            _ => return Err("load is \"first\", \"last\" or an object with position, before, after, before_patches and after_patches".into()),
        };
        let mut rules = LoadRules::default();
        for (key, value) in obj {
            match key.as_str() {
                "position" => rules.position = Some(LoadRules::position_from(value.as_str().ok_or("load: position is \"first\" or \"last\"")?)?),
                "before" | "after" => {
                    let ids = value
                        .as_array()
                        .and_then(|a| a.iter().map(|x| x.as_str().map(str::to_string)).collect::<Option<Vec<_>>>())
                        .ok_or_else(|| format!("load: {key} is a list of mod ids"))?;
                    if key == "before" { rules.before = ids } else { rules.after = ids }
                }
                "before_patches" | "after_patches" => {
                    let on = value.as_bool().ok_or_else(|| format!("load: {key} is true or false"))?;
                    if key == "before_patches" { rules.before_patches = on } else { rules.after_patches = on }
                }
                other => return Err(format!("load: {other} is not a field; use position, before, after, before_patches or after_patches")),
            }
        }
        Ok(rules)
    }
}

impl From<LoadRules> for Value {
    fn from(r: LoadRules) -> Value {
        let pos = r.position.map(|p| if p == Position::First { "first" } else { "last" });
        if r.before.is_empty() && r.after.is_empty() && !r.before_patches && !r.after_patches && let Some(p) = pos {
            return Value::String(p.into());
        }
        let mut o = serde_json::Map::new();
        if let Some(p) = pos {
            o.insert("position".into(), p.into());
        }
        if !r.before.is_empty() {
            o.insert("before".into(), r.before.into());
        }
        if !r.after.is_empty() {
            o.insert("after".into(), r.after.into());
        }
        if r.before_patches {
            o.insert("before_patches".into(), true.into());
        }
        if r.after_patches {
            o.insert("after_patches".into(), true.into());
        }
        Value::Object(o)
    }
}

impl Serialize for LoadRules {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Value::from(self.clone()).serialize(s)
    }
}

impl<'de> Deserialize<'de> for LoadRules {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        LoadRules::try_from(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
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

/// The Obsidian this is, for `min_obsidian`.
fn running_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("the crate's own version")
}

impl Manifest {
    /// Ships with Obsidian: always in the library, can't be removed.
    pub fn validate(&self, mod_dir: &Path) -> Result<()> {
        self.save_editor()?;
        if self.obsidian != 1 {
            return Err(bad("obsidian", "only version 1 is supported"));
        }
        if let Some(needs) = self.min_obsidian.as_ref().filter(|v| **v > running_version()) {
            return Err(ObsidianError::NewerObsidian { needs: needs.to_string() });
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
        if self.every_player && self.scope == Scope::Expansion {
            return Err(bad("every_player", "the content of an expansion already runs only when every player runs it, so leave it out"));
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
        for id in &self.load_after {
            if !valid_id(id) || *id == self.id {
                return Err(bad("load_after", format!("{id} is not the id of another mod")));
            }
        }
        if let Some(rules) = &self.load {
            for (field, ids) in [("load.before", &rules.before), ("load.after", &rules.after)] {
                for id in ids {
                    if !valid_id(id) || *id == self.id {
                        return Err(bad(field, format!("{id} is not the id of another mod")));
                    }
                }
            }
            if let Some(id) = rules.before.iter().find(|id| rules.after.contains(id) || self.load_after.contains(id)) {
                return Err(bad("load", format!("{id} is both in before and in after")));
            }
            if rules.before_patches && rules.after_patches {
                return Err(bad("load", "before_patches and after_patches can't both be true"));
            }
        }
        let options = self.option_specs()?;
        for (i, k) in crate::options::parse_keybinds(&self.keybinds)?.iter().enumerate() {
            let Some(pad) = &k.pad else { continue };
            if !options.iter().any(|o| &o.id == pad && matches!(o.kind, crate::options::OptionKind::Choice(_))) {
                return Err(bad(format!("keybinds[{i}]"), format!("pad: {pad} is not a choice option of this mod")));
            }
        }
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
        crate::parts::check(self, &options)?;
        crate::option_layout::check(self, mod_dir, &options, |p, field| check_path(mod_dir, p, field))?;
        Ok(())
    }

    /// `load` with `load_after` folded in.
    pub fn load_rules(&self) -> LoadRules {
        let mut r = self.load.clone().unwrap_or_default();
        for id in &self.load_after {
            if !r.after.contains(id) {
                r.after.push(id.clone());
            }
        }
        r
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
    // An older mod's manifest is Obsidian's own, and its "session" was an earlier version's guess.
    if m.scope == Scope::Session && m.extra.get("legacy") == Some(&Value::Bool(true)) {
        m.scope = Scope::Unspecified;
    }
    // Asking to run only while every player runs it is asking to change the shared game.
    if m.every_player && m.scope == Scope::Unspecified {
        m.scope = Scope::Session;
    }
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
    fn a_mod_for_a_newer_obsidian_asks_for_an_update() {
        let now = running_version();
        let newer = semver::Version::new(now.major, now.minor + 1, 0);
        let later = EXAMPLE.replace(r#""loader_api": 1,"#, &format!(r#""loader_api": 1, "min_obsidian": "{newer}","#));
        assert!(matches!(load_manifest(mod_with(&later).path()), Err(ObsidianError::NewerObsidian { needs }) if needs == newer.to_string()));
        let fine = EXAMPLE.replace(r#""loader_api": 1,"#, &format!(r#""loader_api": 1, "min_obsidian": "{now}","#));
        assert_eq!(load_manifest(mod_with(&fine).path()).unwrap().min_obsidian, Some(now));
    }

    #[test]
    fn unknown_or_missing_scope_is_unspecified_and_runs_as_client() {
        let weird = EXAMPLE.replace(r#""scope": "client""#, r#""scope": "weird""#);
        assert_eq!(load_manifest(mod_with(&weird).path()).unwrap().scope, Scope::Unspecified);
        let missing = EXAMPLE.replace(r#""scope": "client","#, "");
        let d = mod_with(&missing);
        let m = load_manifest(d.path()).unwrap();
        assert_eq!(m.scope, Scope::Unspecified);
        assert!(m.scope.is_client());
        write_manifest(d.path(), &m).unwrap();
        let written = std::fs::read_to_string(d.path().join(FILE)).unwrap();
        assert!(!written.contains("scope"), "a scope nobody gave isn't written down");
        assert_eq!(load_manifest(d.path()).unwrap().scope, Scope::Unspecified);
        let session = EXAMPLE.replace(r#""scope": "client""#, r#""scope": "session""#);
        assert_eq!(load_manifest(mod_with(&session).path()).unwrap().scope, Scope::Session);
        let library = EXAMPLE.replace(r#""scope": "client""#, r#""scope": "library""#);
        assert_eq!(load_manifest(mod_with(&library).path()).unwrap().scope, Scope::Library);
    }

    #[test]
    fn an_expansion_parses_round_trips_and_isnt_a_client_mod() {
        let json = EXAMPLE.replace(r#""scope": "client""#, r#""scope": "expansion""#);
        let d = mod_with(&json);
        let m = load_manifest(d.path()).unwrap();
        assert_eq!(m.scope, Scope::Expansion);
        assert_eq!(m.scope.name(), "expansion");
        assert!(!m.scope.is_client(), "shared options need a deploy, as in a session mod");
        write_manifest(d.path(), &m).unwrap();
        assert!(std::fs::read_to_string(d.path().join(FILE)).unwrap().contains(r#""scope": "expansion""#));
        assert_eq!(load_manifest(d.path()).unwrap(), m);
    }

    #[test]
    fn an_expansion_doesnt_take_every_player() {
        let both = EXAMPLE.replace(r#""scope": "client","#, r#""scope": "expansion", "every_player": true,"#);
        assert_eq!(field_of(load_manifest(mod_with(&both).path())), "every_player");
    }

    #[test]
    fn an_older_mod_imported_as_session_is_unspecified() {
        let legacy = EXAMPLE.replace(r#""scope": "client","#, r#""scope": "session", "legacy": true,"#);
        assert_eq!(load_manifest(mod_with(&legacy).path()).unwrap().scope, Scope::Unspecified);
        let audio = EXAMPLE.replace(r#""scope": "client","#, r#""scope": "client", "legacy": true,"#);
        assert_eq!(load_manifest(mod_with(&audio).path()).unwrap().scope, Scope::Client, "a sound mod stays client");
    }

    #[test]
    fn every_player_without_a_scope_is_session() {
        let every = EXAMPLE.replace(r#""scope": "client","#, r#""every_player": true,"#);
        assert_eq!(load_manifest(mod_with(&every).path()).unwrap().scope, Scope::Session);
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
    fn a_keybind_names_a_choice_option_as_its_controller_input() {
        let with_pad = |pad: &str| {
            EXAMPLE
                .replace(r#""default": "f8""#, &format!(r#""default": "f8", "pad": {pad}"#))
                .replace(r#"{ "id": "distance", "type": "number", "default": 5 }"#,
                    r#"{ "id": "distance", "type": "number", "default": 5 }, { "id": "pad_toggle", "type": "choice", "choices": ["thumbs", "off"], "default": "thumbs" }"#)
        };
        let m = load_manifest(mod_with(&with_pad(r#""pad_toggle""#)).path()).unwrap();
        assert_eq!(crate::options::parse_keybinds(&m.keybinds).unwrap()[0].pad.as_deref(), Some("pad_toggle"));
        assert_eq!(field_of(load_manifest(mod_with(&with_pad(r#""nothing""#)).path())), "keybinds[0]", "no such option");
        assert_eq!(field_of(load_manifest(mod_with(&with_pad(r#""distance""#)).path())), "keybinds[0]", "not a choice");
        assert_eq!(field_of(load_manifest(mod_with(&with_pad("5")).path())), "keybinds[0]", "not an id");
        let plain = load_manifest(mod_with(EXAMPLE).path()).unwrap();
        assert_eq!(crate::options::parse_keybinds(&plain.keybinds).unwrap()[0].pad, None);
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
    fn load_reads_the_short_text_and_the_rules() {
        let with = |load: &str| EXAMPLE.replace(r#""loader_api": 1,"#, &format!(r#""loader_api": 1, "load": {load},"#));
        let last = load_manifest(mod_with(&with(r#""last""#)).path()).unwrap();
        assert_eq!(last.load, Some(LoadRules { position: Some(Position::Last), ..Default::default() }));
        let first = load_manifest(mod_with(&with(r#""first""#)).path()).unwrap();
        assert_eq!(first.load.unwrap().position, Some(Position::First));

        let d = mod_with(&with(r#"{ "position": "last", "before": ["a.b"], "after": ["c.d"], "before_patches": true }"#));
        let m = load_manifest(d.path()).unwrap();
        let rules = m.load.clone().unwrap();
        assert_eq!((rules.position, rules.before.as_slice(), rules.after.as_slice(), rules.before_patches, rules.after_patches), (Some(Position::Last), &["a.b".to_string()][..], &["c.d".to_string()][..], true, false));
        write_manifest(d.path(), &m).unwrap();
        assert_eq!(load_manifest(d.path()).unwrap(), m, "it survives a round trip");

        // A position alone is written as the text older Obsidian reads.
        let d = mod_with(&with(r#"{ "position": "last" }"#));
        let m = load_manifest(d.path()).unwrap();
        write_manifest(d.path(), &m).unwrap();
        assert!(std::fs::read_to_string(d.path().join(FILE)).unwrap().contains(r#""load": "last""#));
    }

    #[test]
    fn load_after_folds_into_the_rules() {
        let json = EXAMPLE.replace(r#""loader_api": 1,"#, r#""loader_api": 1, "load": { "after": ["x.y"] }, "load_after": ["x.y", "z.w"],"#);
        let m = load_manifest(mod_with(&json).path()).unwrap();
        assert_eq!(m.load_rules().after, ["x.y", "z.w"]);
    }

    #[test]
    fn load_is_checked() {
        for (load, why) in [
            (r#""middle""#, "position"),
            (r#"{ "position": "top" }"#, "position"),
            (r#"{ "befor": ["a.b"] }"#, "befor"),
            (r#"{ "before": "a.b" }"#, "list"),
            (r#"{ "before_patches": "yes" }"#, "true or false"),
            (r#"{ "before": ["Not An Id"] }"#, "not the id"),
            (r#"{ "after": ["author.short-name"] }"#, "not the id"),
            (r#"{ "before": ["a.b"], "after": ["a.b"] }"#, "both"),
            (r#"{ "before_patches": true, "after_patches": true }"#, "both be true"),
            ("3", "object"),
        ] {
            let json = EXAMPLE.replace(r#""loader_api": 1,"#, &format!(r#""loader_api": 1, "load": {load},"#));
            let e = load_manifest(mod_with(&json).path()).unwrap_err().to_string();
            assert!(e.contains(why), "{load}: {e}");
        }
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
