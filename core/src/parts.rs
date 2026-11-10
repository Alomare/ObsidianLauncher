//! Parts of a mod (`parts` in the manifest): pieces that run with a scope of their own, so a session
//! mod can keep what only this player sees running in any lobby. A part with `option` is on only
//! while that toggle option is true. A part that is off runs no code, deploys no file and counts for
//! nothing players compare. Untagged code, options and files follow the strictest part that is on.
//! A mod with no parts runs at the top-level scope, exactly as before parts existed.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::manifest::{Manifest, PartSpec, Scope};
use crate::options::{OptionKind, OptionSpec, is_key_name};
use crate::{ObsidianError, Result};

fn bad(field: impl Into<String>, reason: impl Into<String>) -> ObsidianError {
    ObsidianError::BadManifest { field: field.into(), reason: reason.into() }
}

impl Manifest {
    pub fn part(&self, id: &str) -> Option<&PartSpec> {
        self.parts.iter().find(|p| p.id == id)
    }

    /// Whether the part is on with these option values.
    pub fn part_on(&self, part: &PartSpec, values: &BTreeMap<String, Value>) -> bool {
        part.option.as_ref().is_none_or(|o| values.get(o) == Some(&Value::Bool(true)))
    }

    /// (a client part is on, a session part is on)
    fn kinds_on(&self, values: &BTreeMap<String, Value>) -> (bool, bool) {
        let on = |scope: Scope| self.parts.iter().any(|p| p.scope == scope && self.part_on(p, values));
        (on(Scope::Client), on(Scope::Session))
    }

    /// The scope the mod deploys with, before a library takes one: session while a session part is on,
    /// client while only client parts are. The top-level scope for a mod with no parts, or none on.
    pub fn scope_with(&self, values: &BTreeMap<String, Value>) -> Scope {
        if self.parts.is_empty() {
            return self.scope;
        }
        match self.kinds_on(values) {
            (_, true) => Scope::Session,
            (true, false) => Scope::Client,
            (false, false) => self.scope,
        }
    }

    /// A client part and a session part are both on: the session parts stop in a lobby that isn't
    /// cleared, and the client parts keep running.
    pub fn hybrid_with(&self, values: &BTreeMap<String, Value>) -> bool {
        self.kinds_on(values) == (true, true)
    }

    /// The scope of what is tagged with `part`, or of something untagged: None while the part is off.
    pub fn tag_scope(&self, part: Option<&str>, values: &BTreeMap<String, Value>) -> Option<Scope> {
        match part.and_then(|p| self.part(p)) {
            Some(p) => self.part_on(p, values).then_some(p.scope),
            None => Some(self.scope_with(values)),
        }
    }

    /// The option switches a session part on and off, so the value changes what the lobby runs.
    fn gates_session_part(&self, option: &str) -> bool {
        self.parts.iter().any(|p| p.scope == Scope::Session && p.option.as_deref() == Some(option))
    }

    /// An option only this player's game uses, so the value stays out of what players compare: marked
    /// `"scope": "client"`, in a client part, or in a part that is off. An option that switches a
    /// session part counts in every case but the last.
    pub fn option_personal(&self, spec: &OptionSpec, values: &BTreeMap<String, Value>) -> bool {
        let Some(scope) = self.tag_scope(spec.part.as_deref(), values) else { return true };
        if self.gates_session_part(&spec.id) {
            return false;
        }
        spec.client || (!self.parts.is_empty() && scope.is_client())
    }

    /// Whether a change of the option needs a deploy: it picks files, switches a part, or counts in what
    /// players compare. Without an entry every option needs one, since only a deploy reads the values.
    pub fn option_needs_deploy(&self, spec: &OptionSpec, values: &BTreeMap<String, Value>) -> bool {
        self.resources.iter().any(|r| r.when.contains_key(&spec.id))
            || self.parts.iter().any(|p| p.option.as_deref() == Some(spec.id.as_str()))
            || (!self.scope_with(values).is_client() && !self.option_personal(spec, values))
            || self.entry.is_none()
    }
}

/// Checks `parts` and every `part` tag: ids as option ids, scopes client or session, a part's option a
/// toggle, and no tag naming a part that doesn't exist.
pub(crate) fn check(m: &Manifest, options: &[OptionSpec]) -> Result<()> {
    let tagged = options.iter().any(|o| o.part.is_some()) || m.resources.iter().any(|r| r.part.is_some());
    if m.parts.is_empty() && !tagged {
        return Ok(());
    }
    match m.scope {
        Scope::Expansion if !m.parts.is_empty() => return Err(bad("parts", "an expansion can't have parts yet")),
        Scope::Library if !m.parts.is_empty() => return Err(bad("parts", "a library takes the scope of the mods that need it, so it has no parts")),
        _ => {}
    }
    for (i, p) in m.parts.iter().enumerate() {
        let field = format!("parts[{i}]");
        if !is_key_name(&p.id) {
            return Err(bad(field, "id must be lowercase letters, digits and _"));
        }
        if m.parts[..i].iter().any(|q| q.id == p.id) {
            return Err(bad(field, format!("duplicate id {}", p.id)));
        }
        if !matches!(p.scope, Scope::Client | Scope::Session) {
            return Err(bad(field, "scope is client or session"));
        }
        if p.scope == Scope::Session && m.scope != Scope::Session {
            return Err(bad(field, "a session part needs \"scope\": \"session\" on the mod, the strictest scope of the parts"));
        }
        if let Some(o) = &p.option
            && !options.iter().any(|s| s.id == *o && s.kind == OptionKind::Toggle)
        {
            return Err(bad(field, format!("option: {o} is not a toggle option of this mod")));
        }
    }
    for (i, o) in options.iter().enumerate() {
        if let Some(p) = o.part.as_deref().filter(|p| m.part(p).is_none()) {
            return Err(bad(format!("options[{i}]"), format!("part: {p} is not in parts")));
        }
    }
    for (i, r) in m.resources.iter().enumerate() {
        if let Some(p) = r.part.as_deref().filter(|p| m.part(p).is_none()) {
            return Err(bad(format!("resources[{i}]"), format!("part: {p} is not in parts")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::load_manifest;
    use crate::testutil::{put, spaced_tempdir};
    use serde_json::json;

    /// A mod like Integrated Third Person: a client part and two session parts, each with a toggle.
    pub(crate) const HYBRID: &str = r#"{
      "obsidian": 1, "id": "a.cam", "version": "1.0.0", "name": { "en": "Cam" }, "scope": "session", "entry": "main.lua",
      "parts": [
        { "id": "view", "scope": "client", "name": { "en": "View" } },
        { "id": "range", "scope": "session", "option": "far", "name": { "en": "Range" } }
      ],
      "options": [
        { "id": "dist", "type": "number", "default": 3, "part": "view", "scope": "client" },
        { "id": "fov", "type": "number", "default": 70, "part": "view" },
        { "id": "far", "type": "toggle", "default": true, "part": "range" },
        { "id": "power", "type": "number", "default": 1, "part": "range" },
        { "id": "loose", "type": "number", "default": 1 }
      ],
      "resources": [
        { "type": "texture", "name": "a/sky", "bundle": "00000000000000aa", "data": "f", "part": "view" },
        { "type": "texture", "name": "a/range", "bundle": "00000000000000aa", "data": "f", "part": "range" }
      ]
    }"#;

    fn load(json: &str) -> Result<Manifest> {
        let d = spaced_tempdir();
        put(d.path(), "obsidian.json", json.as_bytes());
        put(d.path(), "main.lua", b"return true\n");
        put(d.path(), "f", b"x");
        load_manifest(d.path())
    }

    fn values(far: bool) -> BTreeMap<String, Value> {
        [("dist", json!(3)), ("fov", json!(70)), ("far", json!(far)), ("power", json!(1)), ("loose", json!(1))].into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    fn reason(r: Result<Manifest>) -> String {
        match r {
            Err(ObsidianError::BadManifest { field, reason }) => format!("{field}: {reason}"),
            other => panic!("expected BadManifest, got {other:?}"),
        }
    }

    #[test]
    fn the_scope_follows_the_parts_that_are_on() {
        let m = load(HYBRID).unwrap();
        assert_eq!(m.parts.len(), 2);
        assert_eq!((m.scope_with(&values(true)), m.hybrid_with(&values(true))), (Scope::Session, true));
        assert_eq!((m.scope_with(&values(false)), m.hybrid_with(&values(false))), (Scope::Client, false), "the session part is off");
        assert_eq!(m.tag_scope(Some("view"), &values(true)), Some(Scope::Client));
        assert_eq!(m.tag_scope(Some("range"), &values(false)), None);
        assert_eq!(m.tag_scope(None, &values(true)), Some(Scope::Session), "untagged follows the strictest part on");
        assert_eq!(m.tag_scope(None, &values(false)), Some(Scope::Client));
    }

    #[test]
    fn options_of_client_parts_and_parts_that_are_off_stay_personal() {
        let m = load(HYBRID).unwrap();
        let specs = m.option_specs().unwrap();
        let personal = |id: &str, far: bool| m.option_personal(specs.iter().find(|s| s.id == id).unwrap(), &values(far));
        let deploy = |id: &str, far: bool| m.option_needs_deploy(specs.iter().find(|s| s.id == id).unwrap(), &values(far));
        assert!(personal("dist", true) && personal("fov", true), "a client part's options");
        assert!(!personal("far", true) && !personal("power", true) && !personal("loose", true));
        assert!(personal("power", false), "a part that is off counts for nothing");
        assert!(personal("loose", false), "untagged follows the client part left");
        assert!(!deploy("fov", true) && deploy("power", true) && deploy("loose", true));
        assert!(deploy("far", true) && deploy("far", false), "switching a part needs a deploy");
    }

    #[test]
    fn a_part_round_trips_with_fields_obsidian_doesnt_know() {
        let json = HYBRID.replace(r#""name": { "en": "Range" }"#, r#""name": { "en": "Range" }, "later": 1"#);
        let d = spaced_tempdir();
        put(d.path(), "obsidian.json", json.as_bytes());
        put(d.path(), "main.lua", b"return true\n");
        put(d.path(), "f", b"x");
        let m = load_manifest(d.path()).unwrap();
        crate::manifest::write_manifest(d.path(), &m).unwrap();
        let again = load_manifest(d.path()).unwrap();
        assert_eq!(again, m);
        assert_eq!(again.parts[1].extra.get("later"), Some(&json!(1)));
        assert_eq!(again.resources[1].part.as_deref(), Some("range"));
    }

    #[test]
    fn parts_and_tags_are_checked() {
        for (from, to, why) in [
            (r#""id": "view""#, r#""id": "View""#, "parts[0]: id must be"),
            (r#""id": "range""#, r#""id": "view""#, "parts[1]: duplicate id view"),
            (r#""scope": "client", "name""#, r#""scope": "library", "name""#, "parts[0]: scope is client or session"),
            (r#""option": "far""#, r#""option": "fov""#, "parts[1]: option: fov is not a toggle"),
            (r#""option": "far""#, r#""option": "gone""#, "parts[1]: option: gone is not a toggle"),
            (r#""default": 1, "part": "range""#, r#""default": 1, "part": "nothing""#, "options[3]: part: nothing is not in parts"),
            (r#""data": "f", "part": "view""#, r#""data": "f", "part": "nothing""#, "resources[0]: part: nothing is not in parts"),
            (r#""scope": "session", "entry""#, r#""scope": "client", "entry""#, "parts[1]: a session part needs"),
            (r#""scope": "session", "entry""#, r#""scope": "expansion", "entry""#, "parts: an expansion can't have parts"),
            (r#""scope": "session", "entry""#, r#""scope": "library", "entry""#, "parts: a library takes"),
        ] {
            let json = HYBRID.replacen(from, to, 1);
            assert_ne!(json, HYBRID, "{from}");
            let got = reason(load(&json));
            assert!(got.starts_with(why), "{got}");
        }
        let no_parts = r#"{ "obsidian": 1, "id": "a.b", "version": "1.0.0", "name": { "en": "B" }, "scope": "session",
            "options": [ { "id": "x", "type": "toggle", "default": true, "part": "view" } ] }"#;
        assert!(reason(load(no_parts)).starts_with("options[0]: part: view is not in parts"));
    }

    #[test]
    fn a_mod_with_no_parts_keeps_the_scope_and_option_rules_it_had() {
        let json = r#"{ "obsidian": 1, "id": "a.b", "version": "1.0.0", "name": { "en": "B" }, "scope": "session", "entry": "main.lua",
            "options": [ { "id": "x", "type": "number", "default": 1 }, { "id": "c", "type": "number", "default": 1, "scope": "client" } ] }"#;
        let m = load(json).unwrap();
        let v: BTreeMap<String, Value> = [("x".to_string(), json!(1)), ("c".to_string(), json!(1))].into();
        let specs = m.option_specs().unwrap();
        assert_eq!((m.scope_with(&v), m.hybrid_with(&v)), (Scope::Session, false));
        assert_eq!(specs.iter().map(|s| m.option_personal(s, &v)).collect::<Vec<_>>(), [false, true]);
        assert_eq!(specs.iter().map(|s| m.option_needs_deploy(s, &v)).collect::<Vec<_>>(), [true, false]);
    }
}
