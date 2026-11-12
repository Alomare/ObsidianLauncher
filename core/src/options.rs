//! Typed mod options (spec 4.1): toggle, choice, number, key and color, validated before a mod sees them.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{ObsidianError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum OptionKind {
    Toggle,
    Choice(Vec<String>),
    Number { min: Option<f64>, max: Option<f64>, step: Option<f64> },
    Key,
    Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionSpec {
    pub id: String,
    pub kind: OptionKind,
    pub default: Value,
    /// `"scope": "client"`: personal, so the value stays out of the mod-set hash.
    pub client: bool,
    pub visible_when: BTreeMap<String, Value>,
    /// The part of the mod the option belongs to (`parts`). Manifest::validate checks that the part exists.
    pub part: Option<String>,
    /// The heading the Options tab lists the option under (`option_groups`). Checked the same way.
    pub group: Option<String>,
}

impl OptionKind {
    pub fn name(&self) -> &'static str {
        match self {
            OptionKind::Toggle => "toggle",
            OptionKind::Choice(_) => "choice",
            OptionKind::Number { .. } => "number",
            OptionKind::Key => "key",
            OptionKind::Color => "color",
        }
    }
}

fn bad(i: usize, reason: impl Into<String>) -> ObsidianError {
    ObsidianError::BadManifest { field: format!("options[{i}]"), reason: reason.into() }
}

pub fn is_key_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

pub(crate) fn is_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_plain(s: &str) -> bool {
    !s.is_empty() && !s.contains(['\t', '\n', '\r'])
}

/// Parses the `options` array of a manifest. Every default must pass the option's own check.
pub fn parse_options(raw: &[Value]) -> Result<Vec<OptionSpec>> {
    let mut out: Vec<OptionSpec> = Vec::new();
    for (i, v) in raw.iter().enumerate() {
        let o = v.as_object().ok_or_else(|| bad(i, "must be an object"))?;
        let id = o.get("id").and_then(Value::as_str).filter(|s| is_key_name(s)).ok_or_else(|| bad(i, "id must be lowercase letters, digits and _"))?;
        if out.iter().any(|x| x.id == id) {
            return Err(bad(i, format!("duplicate id {id}")));
        }
        let num = |k: &str| o.get(k).and_then(Value::as_f64);
        let kind = match o.get("type").and_then(Value::as_str) {
            Some("toggle") => OptionKind::Toggle,
            Some("number") => OptionKind::Number { min: num("min"), max: num("max"), step: num("step").filter(|s| *s > 0.0) },
            Some("key") => OptionKind::Key,
            Some("color") => OptionKind::Color,
            Some("choice") => {
                let choices = o.get("choices").and_then(Value::as_array).ok_or_else(|| bad(i, "a choice needs choices"))?;
                let ids: Vec<String> = choices
                    .iter()
                    .map(|c| c.as_str().or_else(|| c.get("id").and_then(Value::as_str)).map(str::to_string))
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad(i, "each choice is a string or has an id"))?;
                if ids.is_empty() || !ids.iter().all(|c| is_plain(c)) {
                    return Err(bad(i, "choices must be non-empty plain text"));
                }
                OptionKind::Choice(ids)
            }
            other => return Err(bad(i, format!("unknown type {other:?}"))),
        };
        let visible_when = match o.get("visible_when") {
            Some(Value::Object(m)) => m.clone().into_iter().collect(),
            Some(_) => return Err(bad(i, "visible_when must be an object")),
            None => BTreeMap::new(),
        };
        let tag = |key: &str| -> Result<Option<String>> {
            match o.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => v.as_str().filter(|s| is_key_name(s)).map(|s| Some(s.to_string())).ok_or_else(|| bad(i, format!("{key} must be an id: lowercase letters, digits and _"))),
            }
        };
        let mut spec = OptionSpec {
            id: id.to_string(),
            kind,
            default: Value::Null,
            client: o.get("scope").and_then(Value::as_str) == Some("client"),
            visible_when,
            part: tag("part")?,
            group: tag("group")?,
        };
        let default = o.get("default").ok_or_else(|| bad(i, "needs a default"))?;
        spec.default = validate(&spec, default).map_err(|_| bad(i, format!("default {default} is not valid for a {}", spec.kind.name())))?;
        out.push(spec);
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeybindSpec {
    pub id: String,
    pub default: String,
    /// The choice option that holds the controller input for the same action, if any: the app shows
    /// both in one row (`"pad": "<option id>"`).
    pub pad: Option<String>,
}

/// Parses the `keybinds` array of a manifest: an id and a default key name each, and optionally the
/// choice option with the controller input (`pad`). Manifest::validate checks that option exists.
pub fn parse_keybinds(raw: &[Value]) -> Result<Vec<KeybindSpec>> {
    let mut out: Vec<KeybindSpec> = Vec::new();
    for (i, v) in raw.iter().enumerate() {
        let bad = |reason: String| ObsidianError::BadManifest { field: format!("keybinds[{i}]"), reason };
        let id = v.get("id").and_then(Value::as_str).filter(|s| is_key_name(s)).ok_or_else(|| bad("id must be lowercase letters, digits and _".into()))?;
        if out.iter().any(|k| k.id == id) {
            return Err(bad(format!("duplicate id {id}")));
        }
        let default = v.get("default").and_then(Value::as_str).filter(|s| is_key_name(s)).ok_or_else(|| bad("default must be a key name such as f8".into()))?;
        let pad = match v.get("pad") {
            None | Some(Value::Null) => None,
            Some(p) => Some(p.as_str().filter(|s| is_key_name(s)).ok_or_else(|| bad("pad must be the id of a choice option".into()))?.to_string()),
        };
        out.push(KeybindSpec { id: id.to_string(), default: default.to_string(), pad });
    }
    Ok(out)
}

/// True when every option named in `when` has the listed value.
pub fn when_matches(when: &BTreeMap<String, Value>, values: &BTreeMap<String, Value>) -> bool {
    when.iter().all(|(id, want)| values.get(id) == Some(want))
}

/// Checks a value against the option, returning the value as stored.
pub fn validate(spec: &OptionSpec, value: &Value) -> Result<Value> {
    let invalid = || ObsidianError::BadOptionValue { option: spec.id.clone(), value: value.to_string() };
    match &spec.kind {
        OptionKind::Toggle => value.as_bool().map(Value::Bool).ok_or_else(invalid),
        OptionKind::Choice(ids) => value.as_str().filter(|s| ids.iter().any(|c| c == s)).map(|s| Value::String(s.into())).ok_or_else(invalid),
        OptionKind::Key => value.as_str().filter(|s| is_key_name(s)).map(|s| Value::String(s.into())).ok_or_else(invalid),
        OptionKind::Color => value.as_str().filter(|s| is_color(s)).map(|s| Value::String(s.to_ascii_lowercase())).ok_or_else(invalid),
        OptionKind::Number { min, max, step } => {
            let n = value.as_f64().filter(|n| n.is_finite()).ok_or_else(invalid)?;
            if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                return Err(invalid());
            }
            if let Some(s) = step {
                let k = (n - min.unwrap_or(0.0)) / s;
                if (k - k.round()).abs() > 1e-6 {
                    return Err(invalid());
                }
            }
            Ok(value.clone())
        }
    }
}

/// The value of every option: the stored one when the stored one still passes the check (same id and
/// type after a mod update), the default otherwise.
pub fn effective_values(specs: &[OptionSpec], stored: Option<&BTreeMap<String, Value>>) -> BTreeMap<String, Value> {
    specs
        .iter()
        .map(|s| {
            let v = stored.and_then(|m| m.get(&s.id)).and_then(|v| validate(s, v).ok()).unwrap_or_else(|| s.default.clone());
            (s.id.clone(), v)
        })
        .collect()
}

/// The value as one line of the settings file: 1 or 0 for toggles, the number, or the text.
pub fn value_text(value: &Value) -> String {
    match value {
        Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn one(v: Value) -> OptionSpec {
        parse_options(&[v]).unwrap().remove(0)
    }

    #[test]
    fn number_outside_range_is_rejected() {
        let s = one(json!({"id": "d", "type": "number", "default": 5, "min": 1, "max": 12, "step": 0.5}));
        assert!(validate(&s, &json!(13)).is_err());
        assert!(validate(&s, &json!(0)).is_err());
        assert!(validate(&s, &json!(2.25)).is_err());
        assert_eq!(validate(&s, &json!(2.5)).unwrap(), json!(2.5));
    }

    #[test]
    fn unknown_choice_is_rejected() {
        let s = one(json!({"id": "c", "type": "choice", "default": "a", "choices": ["a", {"id": "b"}]}));
        assert!(validate(&s, &json!("z")).is_err());
        assert_eq!(validate(&s, &json!("b")).unwrap(), json!("b"));
    }

    #[test]
    fn bad_default_is_a_manifest_error() {
        let e = parse_options(&[json!({"id": "t", "type": "toggle", "default": 3})]).unwrap_err();
        assert!(matches!(e, ObsidianError::BadManifest { ref field, .. } if field == "options[0]"), "{e}");
    }

    #[test]
    fn update_keeps_values_with_same_id_and_type() {
        let old: BTreeMap<String, Value> = [("d".to_string(), json!(7)), ("t".to_string(), json!(true))].into();
        let specs = parse_options(&[
            json!({"id": "d", "type": "number", "default": 5, "min": 1, "max": 12}),
            json!({"id": "t", "type": "choice", "default": "x", "choices": ["x", "y"]}),
        ])
        .unwrap();
        let v = effective_values(&specs, Some(&old));
        assert_eq!(v["d"], json!(7));
        assert_eq!(v["t"], json!("x"), "type changed, so back to the default");
    }

    #[test]
    fn client_scope_and_value_text() {
        let s = one(json!({"id": "cam", "type": "toggle", "default": false, "scope": "client"}));
        assert!(s.client);
        assert_eq!(value_text(&json!(true)), "1");
        assert_eq!(value_text(&json!(2.5)), "2.5");
        assert_eq!(value_text(&json!("#ff00aa")), "#ff00aa");
    }
}
