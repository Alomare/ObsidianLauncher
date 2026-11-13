//! Hybrid Scopes: `secondary_scopes` in the manifest (`parts` before Hybrid Scopes had that name). Code,
//! options and files tagged with a secondary scope run with that scope instead of the primary `scope`,
//! so a session mod keeps what only this player sees running in any lobby, and an expansion can change
//! the shared game in a cleared lobby outside its content. One with `option` is on only while that
//! toggle option is true. One that is off runs no code, deploys no file and counts for nothing players
//! compare. In a session mod, untagged code, options and files follow the strictest secondary scope that
//! is on; in an expansion, they stay with the expansion. A mod without secondary scopes runs at the
//! primary scope, which is also all an older Obsidian reads.

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

    /// The scope the mod deploys with, before a library takes one: session while a session secondary
    /// scope is on, client while only client ones are. The primary scope for a mod without secondary
    /// scopes, or none on, and for an expansion, which stays one ([`Manifest::session_outside_content`]).
    pub fn scope_with(&self, values: &BTreeMap<String, Value>) -> Scope {
        if self.parts.is_empty() || self.scope == Scope::Expansion {
            return self.scope;
        }
        match self.kinds_on(values) {
            (_, true) => Scope::Session,
            (true, false) => Scope::Client,
            (false, false) => self.scope,
        }
    }

    /// A client and a session secondary scope are both on: the session one stops in a lobby that isn't
    /// cleared, and the client one keeps running. Not for an expansion, which stays one.
    pub fn hybrid_with(&self, values: &BTreeMap<String, Value>) -> bool {
        self.scope != Scope::Expansion && self.kinds_on(values) == (true, true)
    }

    /// An expansion with a session secondary scope on: that code runs in a cleared lobby outside the
    /// content too, so the deploy counts it as a session mod's.
    pub fn session_outside_content(&self, values: &BTreeMap<String, Value>) -> bool {
        self.scope == Scope::Expansion && self.kinds_on(values).1
    }

    /// The scope of what is tagged with the secondary scope `part`, or of something untagged: None while it is off.
    pub fn tag_scope(&self, part: Option<&str>, values: &BTreeMap<String, Value>) -> Option<Scope> {
        match part.and_then(|p| self.part(p)) {
            Some(p) => self.part_on(p, values).then_some(p.scope),
            None => Some(self.scope_with(values)),
        }
    }

    /// The option switches a session secondary scope on and off, so the value changes what the lobby runs.
    fn gates_session_part(&self, option: &str) -> bool {
        self.parts.iter().any(|p| p.scope == Scope::Session && p.option.as_deref() == Some(option))
    }

    /// An option only this player's game uses, so the value stays out of what players compare: marked
    /// `"scope": "client"`, in a client secondary scope, or in one that is off. An option that switches a
    /// session secondary scope counts in every case but the last.
    pub fn option_personal(&self, spec: &OptionSpec, values: &BTreeMap<String, Value>) -> bool {
        let Some(scope) = self.tag_scope(spec.part.as_deref(), values) else { return true };
        if self.gates_session_part(&spec.id) {
            return false;
        }
        spec.client || (!self.parts.is_empty() && scope.is_client())
    }

    /// Whether a change of the option needs a deploy: it picks files, switches a secondary scope, or counts in what
    /// players compare. Without an entry every option needs one, since only a deploy reads the values.
    pub fn option_needs_deploy(&self, spec: &OptionSpec, values: &BTreeMap<String, Value>) -> bool {
        self.resources.iter().any(|r| r.when.contains_key(&spec.id))
            || self.parts.iter().any(|p| p.option.as_deref() == Some(spec.id.as_str()))
            || (!self.scope_with(values).is_client() && !self.option_personal(spec, values))
            || self.entry.is_none()
    }
}

/// Checks `secondary_scopes` and every `secondary_scope` tag: ids as option ids, scopes client or
/// session, the option of one a toggle, and no tag naming one that doesn't exist.
pub(crate) fn check(m: &Manifest, options: &[OptionSpec]) -> Result<()> {
    let tagged = options.iter().any(|o| o.part.is_some()) || m.resources.iter().any(|r| r.part.is_some());
    if m.parts.is_empty() && !tagged {
        return Ok(());
    }
    if m.scope == Scope::Library && !m.parts.is_empty() {
        return Err(bad("secondary_scopes", "a library takes the scope of the mods that need it, so it has no secondary scopes"));
    }
    for (i, p) in m.parts.iter().enumerate() {
        let field = format!("secondary_scopes[{i}]");
        if !is_key_name(&p.id) {
            return Err(bad(field, "id must be lowercase letters, digits and _"));
        }
        if m.parts[..i].iter().any(|q| q.id == p.id) {
            return Err(bad(field, format!("duplicate id {}", p.id)));
        }
        if !matches!(p.scope, Scope::Client | Scope::Session) {
            return Err(bad(field, "scope is client or session"));
        }
        if p.scope == Scope::Session && !matches!(m.scope, Scope::Session | Scope::Expansion) {
            return Err(bad(field, "a session secondary scope needs \"scope\": \"session\" or \"expansion\" as the primary scope"));
        }
        if let Some(o) = &p.option
            && !options.iter().any(|s| s.id == *o && s.kind == OptionKind::Toggle)
        {
            return Err(bad(field, format!("option: {o} is not a toggle option of this mod")));
        }
    }
    for (i, o) in options.iter().enumerate() {
        if let Some(p) = o.part.as_deref().filter(|p| m.part(p).is_none()) {
            return Err(bad(format!("options[{i}]"), format!("secondary_scope: {p} is not in secondary_scopes")));
        }
    }
    for (i, r) in m.resources.iter().enumerate() {
        if let Some(p) = r.part.as_deref().filter(|p| m.part(p).is_none()) {
            return Err(bad(format!("resources[{i}]"), format!("secondary_scope: {p} is not in secondary_scopes")));
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
            (r#""id": "view""#, r#""id": "View""#, "secondary_scopes[0]: id must be"),
            (r#""id": "range""#, r#""id": "view""#, "secondary_scopes[1]: duplicate id view"),
            (r#""scope": "client", "name""#, r#""scope": "library", "name""#, "secondary_scopes[0]: scope is client or session"),
            (r#""option": "far""#, r#""option": "fov""#, "secondary_scopes[1]: option: fov is not a toggle"),
            (r#""option": "far""#, r#""option": "gone""#, "secondary_scopes[1]: option: gone is not a toggle"),
            (r#""default": 1, "part": "range""#, r#""default": 1, "part": "nothing""#, "options[3]: secondary_scope: nothing is not in secondary_scopes"),
            (r#""data": "f", "part": "view""#, r#""data": "f", "part": "nothing""#, "resources[0]: secondary_scope: nothing is not in secondary_scopes"),
            (r#""scope": "session", "entry""#, r#""scope": "client", "entry""#, "secondary_scopes[1]: a session secondary scope needs"),
            (r#""scope": "session", "entry""#, r#""scope": "library", "entry""#, "secondary_scopes: a library takes"),
        ] {
            let json = HYBRID.replacen(from, to, 1);
            assert_ne!(json, HYBRID, "{from}");
            let got = reason(load(&json));
            assert!(got.starts_with(why), "{got}");
        }
        let no_parts = r#"{ "obsidian": 1, "id": "a.b", "version": "1.0.0", "name": { "en": "B" }, "scope": "session",
            "options": [ { "id": "x", "type": "toggle", "default": true, "part": "view" } ] }"#;
        assert!(reason(load(no_parts)).starts_with("options[0]: secondary_scope: view is not in secondary_scopes"));
    }

    /// HYBRID with the names Hybrid Scopes has now. The old names in HYBRID still read the same.
    fn named(json: &str) -> String {
        json.replace(r#""parts":"#, r#""secondary_scopes":"#).replace(r#""part":"#, r#""secondary_scope":"#)
    }

    #[test]
    fn the_new_names_and_the_old_ones_read_the_same() {
        let new = load(&named(HYBRID)).unwrap();
        let old = load(HYBRID).unwrap();
        assert_eq!(new.parts, old.parts);
        assert_eq!(new.resources.iter().map(|r| r.part.clone()).collect::<Vec<_>>(), old.resources.iter().map(|r| r.part.clone()).collect::<Vec<_>>());
        assert_eq!(new.option_specs().unwrap(), old.option_specs().unwrap());
        assert_eq!(new.parts.len(), 2);
    }

    #[test]
    fn an_expansion_stays_one_and_a_session_secondary_scope_counts_outside_the_content() {
        let json = named(HYBRID).replacen(r#""scope": "session", "entry""#, r#""scope": "expansion", "entry""#, 1);
        let m = load(&json).unwrap();
        for far in [true, false] {
            assert_eq!(m.scope_with(&values(far)), Scope::Expansion);
            assert!(!m.hybrid_with(&values(far)));
            assert_eq!(m.session_outside_content(&values(far)), far);
        }
        // Untagged things stay with the expansion; tagged ones take their secondary scope.
        assert_eq!(m.tag_scope(None, &values(true)), Some(Scope::Expansion));
        assert_eq!(m.tag_scope(Some("range"), &values(true)), Some(Scope::Session));
        assert_eq!(m.tag_scope(Some("range"), &values(false)), None);
        assert_eq!(m.tag_scope(Some("view"), &values(false)), Some(Scope::Client));
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
