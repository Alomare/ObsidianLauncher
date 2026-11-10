//! How the app lays out a mod's options: collapsible headings (`option_groups`, with `group` on each
//! option) and live pictures of the values (`option_previews`, SVG templates the app fills in). Older
//! Obsidian ignores both fields.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::manifest::{Localized, Manifest};
use crate::options::{OptionSpec, is_key_name};
use crate::{ObsidianError, Result};

/// The largest preview template, in bytes.
pub const MAX_TEMPLATE: u64 = 64 * 1024;
const DEFAULT_SIZE: (f64, f64) = (240.0, 140.0);
const MAX_SIDE: f64 = 2048.0;

#[derive(Debug, Clone, PartialEq)]
pub struct OptionGroup {
    pub id: String,
    pub label: Localized,
    pub description: Localized,
    /// Folded the first time the player sees it.
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionPreview {
    pub id: String,
    /// The group it shows at the top of, or the top of the Options tab.
    pub group: Option<String>,
    /// An SVG file in the mod folder.
    pub template: PathBuf,
    pub width: f64,
    pub height: f64,
    pub label: Localized,
}

fn bad(field: impl Into<String>, reason: impl Into<String>) -> ObsidianError {
    ObsidianError::BadManifest { field: field.into(), reason: reason.into() }
}

fn localized(v: Option<&Value>) -> Localized {
    v.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

/// The objects of a list field, each with the index and an id that follows the option id rules.
fn entries<'a>(m: &'a Manifest, key: &str) -> Result<Vec<(String, &'a serde_json::Map<String, Value>, String)>> {
    let Some(raw) = m.extra.get(key) else { return Ok(Vec::new()) };
    let list = raw.as_array().ok_or_else(|| bad(key, "must be a list"))?;
    let mut out: Vec<(String, &serde_json::Map<String, Value>, String)> = Vec::new();
    for (i, v) in list.iter().enumerate() {
        let field = format!("{key}[{i}]");
        let o = v.as_object().ok_or_else(|| bad(&field, "must be an object"))?;
        let id = o.get("id").and_then(Value::as_str).filter(|s| is_key_name(s)).ok_or_else(|| bad(&field, "id must be lowercase letters, digits and _"))?;
        if out.iter().any(|(_, _, seen)| seen == id) {
            return Err(bad(&field, format!("duplicate id {id}")));
        }
        out.push((field, o, id.to_string()));
    }
    Ok(out)
}

impl Manifest {
    /// `option_groups`, in the order declared.
    pub fn option_groups(&self) -> Result<Vec<OptionGroup>> {
        entries(self, "option_groups")?
            .into_iter()
            .map(|(field, o, id)| {
                let collapsed = match o.get("collapsed") {
                    None | Some(Value::Null) => false,
                    Some(v) => v.as_bool().ok_or_else(|| bad(&field, "collapsed is true or false"))?,
                };
                Ok(OptionGroup { id, label: localized(o.get("label")), description: localized(o.get("description")), collapsed })
            })
            .collect()
    }

    /// `option_previews`, in the order declared. The template files are checked by `check`.
    pub fn option_previews(&self) -> Result<Vec<OptionPreview>> {
        entries(self, "option_previews")?
            .into_iter()
            .map(|(field, o, id)| {
                let template = o.get("template").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or_else(|| bad(&field, "template is the path of an SVG file in the mod"))?;
                let side = |key: &str, default: f64| -> Result<f64> {
                    match o.get(key) {
                        None | Some(Value::Null) => Ok(default),
                        Some(v) => v.as_f64().filter(|n| *n >= 1.0 && *n <= MAX_SIDE).ok_or_else(|| bad(&field, format!("{key} is a number from 1 to {MAX_SIDE}"))),
                    }
                };
                let group = match o.get("group") {
                    None | Some(Value::Null) => None,
                    Some(g) => Some(g.as_str().ok_or_else(|| bad(&field, "group is the id of an option group"))?.to_string()),
                };
                Ok(OptionPreview { id, group, template: PathBuf::from(template), width: side("width", DEFAULT_SIZE.0)?, height: side("height", DEFAULT_SIZE.1)?, label: localized(o.get("label")) })
            })
            .collect()
    }
}

/// The text of a preview template when the file is SVG and small enough; the reason otherwise.
fn template_text(path: &Path) -> std::result::Result<String, String> {
    if path.extension().is_none_or(|x| !x.eq_ignore_ascii_case("svg")) {
        return Err("template must be an .svg file".into());
    }
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_TEMPLATE {
        return Err(format!("template is {size} bytes, more than the {MAX_TEMPLATE} allowed"));
    }
    let text = String::from_utf8(std::fs::read(path).map_err(|e| e.to_string())?).map_err(|_| "template is not UTF-8 text".to_string())?;
    if !is_svg(&text) {
        return Err("template is not an SVG document".into());
    }
    Ok(text)
}

/// The document starts with an `<svg` element, after an XML declaration, comments or a doctype.
fn is_svg(text: &str) -> bool {
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    loop {
        if let Some(r) = rest.strip_prefix("<?") {
            let Some(end) = r.find("?>") else { return false };
            rest = r[end + 2..].trim_start();
        } else if let Some(r) = rest.strip_prefix("<!--") {
            let Some(end) = r.find("-->") else { return false };
            rest = r[end + 3..].trim_start();
        } else if let Some(r) = rest.strip_prefix("<!") {
            let Some(end) = r.find('>') else { return false };
            rest = r[end + 1..].trim_start();
        } else {
            break;
        }
    }
    rest.strip_prefix("<svg").and_then(|r| r.chars().next()).is_some_and(|c| c.is_whitespace() || c == '>' || c == '/')
}

/// Checks the groups and previews against the options and the mod folder: every `group` names a
/// declared group, and every template is an SVG file of at most 64 KB inside the folder.
pub(crate) fn check(m: &Manifest, mod_dir: &Path, options: &[OptionSpec], check_path: impl Fn(&Path, &str) -> Result<()>) -> Result<()> {
    let groups = m.option_groups()?;
    let known = |g: &str| groups.iter().any(|x| x.id == g);
    for (i, o) in options.iter().enumerate() {
        if let Some(g) = o.group.as_deref().filter(|g| !known(g)) {
            return Err(bad(format!("options[{i}]"), format!("group: {g} is not in option_groups")));
        }
    }
    for (i, p) in m.option_previews()?.iter().enumerate() {
        let field = format!("option_previews[{i}]");
        if let Some(g) = p.group.as_deref().filter(|g| !known(g)) {
            return Err(bad(field, format!("group: {g} is not in option_groups")));
        }
        check_path(&p.template, &field)?;
        template_text(&mod_dir.join(&p.template)).map_err(|why| bad(&field, why))?;
    }
    Ok(())
}

/// A file's time and size: a cached template stays good while they are the same.
type Stamp = (std::time::SystemTime, u64);

fn cache() -> &'static std::sync::Mutex<BTreeMap<PathBuf, (Stamp, Option<String>)>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<BTreeMap<PathBuf, (Stamp, Option<String>)>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The template of a preview of the mod in `mod_dir`, read once while the file is unchanged: the app's
/// snapshot asks after every click. None when the file is gone or no longer a valid template.
pub fn template(mod_dir: &Path, preview: &OptionPreview) -> Option<String> {
    let path = mod_dir.join(&preview.template);
    let meta = std::fs::metadata(&path).ok()?;
    let stamp = (meta.modified().ok()?, meta.len());
    if let Some((s, text)) = cache().lock().unwrap().get(&path)
        && *s == stamp
    {
        return text.clone();
    }
    let text = template_text(&path).ok();
    cache().lock().unwrap().insert(path, (stamp, text.clone()));
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::load_manifest;
    use crate::testutil::{put, spaced_tempdir};

    const SVG: &[u8] = br#"<?xml version="1.0"?><!-- a cross --><svg xmlns="http://www.w3.org/2000/svg" width="240" height="140"><circle r="{{size}}"/></svg>"#;

    const MOD: &str = r##"{ "obsidian": 1, "id": "a.cross", "version": "1.0.0", "name": { "en": "Cross" }, "scope": "client",
      "option_groups": [ { "id": "idle", "label": { "en": "Idle" }, "collapsed": true }, { "id": "aim", "label": { "en": "Aiming" } } ],
      "option_previews": [ { "id": "idle_preview", "group": "idle", "template": "preview/idle.svg", "width": 200, "height": 100, "label": { "en": "Preview" } } ],
      "options": [
        { "id": "size", "type": "number", "default": 4, "group": "idle" },
        { "id": "color", "type": "color", "default": "#ffffff" }
      ] }"##;

    fn folder(json: &str, svg: &[u8]) -> tempfile::TempDir {
        let d = spaced_tempdir();
        put(d.path(), "obsidian.json", json.as_bytes());
        put(d.path(), "preview/idle.svg", svg);
        d
    }

    fn reason(r: Result<Manifest>) -> String {
        match r {
            Err(ObsidianError::BadManifest { field, reason }) => format!("{field}: {reason}"),
            other => panic!("expected BadManifest, got {other:?}"),
        }
    }

    #[test]
    fn groups_and_previews_parse_and_the_template_is_read() {
        let d = folder(MOD, SVG);
        let m = load_manifest(d.path()).unwrap();
        let groups = m.option_groups().unwrap();
        assert_eq!(groups.iter().map(|g| (g.id.as_str(), g.collapsed)).collect::<Vec<_>>(), [("idle", true), ("aim", false)]);
        assert_eq!(groups[0].label["en"], "Idle");
        let previews = m.option_previews().unwrap();
        assert_eq!((previews[0].group.as_deref(), previews[0].width, previews[0].height), (Some("idle"), 200.0, 100.0));
        assert_eq!(m.option_specs().unwrap()[0].group.as_deref(), Some("idle"));
        assert!(template(d.path(), &previews[0]).unwrap().contains("{{size}}"));
        crate::manifest::write_manifest(d.path(), &m).unwrap();
        assert_eq!(load_manifest(d.path()).unwrap(), m, "both fields survive a round trip");
    }

    #[test]
    fn a_preview_without_a_size_takes_the_default() {
        let json = MOD.replace(r#", "width": 200, "height": 100"#, "");
        let p = &load_manifest(folder(&json, SVG).path()).unwrap().option_previews().unwrap()[0];
        assert_eq!((p.width, p.height), DEFAULT_SIZE);
    }

    #[test]
    fn groups_previews_and_templates_are_checked() {
        for (from, to, why) in [
            (r#""group": "idle" }"#, r#""group": "gone" }"#, "options[0]: group: gone is not in option_groups"),
            (r#""group": "idle", "template""#, r#""group": "gone", "template""#, "option_previews[0]: group: gone"),
            (r#""id": "aim""#, r#""id": "idle""#, "option_groups[1]: duplicate id idle"),
            (r#""id": "aim""#, r#""id": "Aim""#, "option_groups[1]: id must be"),
            (r#""collapsed": true"#, r#""collapsed": "yes""#, "option_groups[0]: collapsed is true or false"),
            ("preview/idle.svg", "preview/missing.svg", "option_previews[0]: preview/missing.svg does not exist"),
            ("preview/idle.svg", "../idle.svg", "option_previews[0]: ../idle.svg must be a path inside"),
            (r#""width": 200"#, r#""width": 0"#, "option_previews[0]: width is a number"),
            (r#""template": "preview/idle.svg", "#, "", "option_previews[0]: template is the path"),
        ] {
            let json = MOD.replacen(from, to, 1);
            assert_ne!(json, MOD, "{from}");
            let got = reason(load_manifest(folder(&json, SVG).path()));
            assert!(got.starts_with(why), "{got}");
        }
        assert!(reason(load_manifest(folder(MOD, b"<html><body/></html>").path())).contains("not an SVG document"));
        let mut big = b"<svg xmlns=\"http://www.w3.org/2000/svg\">".to_vec();
        big.resize(MAX_TEMPLATE as usize + 1, b' ');
        assert!(reason(load_manifest(folder(MOD, &big).path())).contains("more than the 65536 allowed"));
        let png = MOD.replace("preview/idle.svg", "preview/idle.png");
        let d = folder(&png, SVG);
        put(d.path(), "preview/idle.png", SVG);
        assert!(reason(load_manifest(d.path())).contains("must be an .svg file"));
    }

    #[test]
    fn svg_is_told_from_other_documents() {
        assert!(is_svg("<svg/>"));
        assert!(is_svg("\u{feff}  <?xml version=\"1.0\"?>\n<!DOCTYPE svg>\n<!-- x --><svg width=\"1\"></svg>"));
        assert!(!is_svg("<svgx/>"));
        assert!(!is_svg("<html><svg/></html>"));
        assert!(!is_svg("<!-- never closed <svg/>"));
    }
}
