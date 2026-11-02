//! Changing a mod's option values and keys in a profile, as the CLI and the app both do, and
//! writing them to the settings file the loader reads at launch.

use std::path::Path;

use serde_json::Value;

use crate::library::{Library, Profile};
use crate::manifest::Manifest;
use crate::options::{parse_keybinds, validate};
use crate::{ObsidianError, Result};

/// When a changed option reaches the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applies {
    /// Through the settings file, at the next launch.
    NextLaunch,
    /// It picks resources or is part of a session mod's set, so it needs a deploy.
    AfterDeploy,
}

fn manifest(lib: &Library, id: &str) -> Result<Manifest> {
    lib.list()?.into_iter().find(|m| m.id == id).ok_or_else(|| ObsidianError::ModMissing(id.to_string()))
}

/// Whether a change to `option` of `m` needs a deploy.
pub fn needs_deploy(m: &Manifest, option: &str, client: bool) -> bool {
    m.resources.iter().any(|r| r.when.contains_key(option)) || (!m.scope.is_client() && !client) || m.entry.is_none()
}

/// Sets an option to `value`, or back to the default when `value` is `None`.
pub fn set_option(lib: &Library, profile: &mut Profile, id: &str, option: &str, value: Option<&Value>) -> Result<Applies> {
    let m = manifest(lib, id)?;
    let specs = m.option_specs()?;
    let spec = specs.iter().find(|s| s.id == option).ok_or_else(|| ObsidianError::UnknownOption { mod_id: id.into(), option: option.into() })?;
    match value {
        Some(v) => {
            let v = validate(spec, v)?;
            profile.options.entry(id.to_string()).or_default().insert(option.to_string(), v);
        }
        None => {
            if let Some(values) = profile.options.get_mut(id) {
                values.remove(option);
                if values.is_empty() {
                    profile.options.remove(id);
                }
            }
        }
    }
    Ok(if needs_deploy(&m, option, spec.client) { Applies::AfterDeploy } else { Applies::NextLaunch })
}

/// Binds a keybind to `key`, or back to the mod's default when `key` is `None`.
pub fn set_key(lib: &Library, profile: &mut Profile, id: &str, keybind: &str, key: Option<&str>) -> Result<()> {
    let m = manifest(lib, id)?;
    if !parse_keybinds(&m.keybinds)?.iter().any(|k| k.id == keybind) {
        return Err(ObsidianError::UnknownKeybind { mod_id: id.into(), keybind: keybind.into() });
    }
    match key {
        Some(k) if crate::options::is_key_name(k) => {
            profile.keys.entry(id.to_string()).or_default().insert(keybind.to_string(), k.to_string());
        }
        Some(k) => return Err(ObsidianError::BadOptionValue { option: keybind.into(), value: k.into() }),
        None => {
            if let Some(keys) = profile.keys.get_mut(id) {
                keys.remove(keybind);
                if keys.is_empty() {
                    profile.keys.remove(id);
                }
            }
        }
    }
    Ok(())
}

/// Writes the profile's consent, option values and keys where the loader reads them.
pub fn write_settings(lib: &Library, profile: &Profile, saves_dir: &Path) -> Result<()> {
    crate::settings::write(&crate::settings::game_side_dir(saves_dir), &crate::loader::settings_for(lib, profile)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::put;

    fn world() -> (tempfile::TempDir, Library, Profile) {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("stage");
        put(&d, "obsidian.json", br##"{"obsidian":1,"id":"opt","version":"1.0.0","name":{"en":"o"},"entry":"main.lua","scope":"client",
            "options":[{"id":"fast","type":"toggle","default":false},{"id":"sky","type":"choice","default":"day","choices":["day","night"]}],
            "keybinds":[{"id":"toggle","default":"f8"}],
            "resources":[{"type":"#0000000000000001","name":"#00000000000000f0","bundle":"00000000000000aa","data":"day.bin","when":{"sky":"day"}}]}"##);
        put(&d, "main.lua", b"return true\n");
        put(&d, "day.bin", b"day");
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&d).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        (t, lib, p)
    }

    #[test]
    fn options_validate_say_when_they_apply_and_reset() {
        let (_t, lib, mut p) = world();
        assert_eq!(set_option(&lib, &mut p, "opt", "fast", Some(&Value::Bool(true))).unwrap(), Applies::NextLaunch);
        assert_eq!(set_option(&lib, &mut p, "opt", "sky", Some(&"night".into())).unwrap(), Applies::AfterDeploy, "sky picks resources");
        assert!(set_option(&lib, &mut p, "opt", "fast", Some(&"yes".into())).is_err());
        assert!(matches!(set_option(&lib, &mut p, "opt", "nope", None), Err(ObsidianError::UnknownOption { .. })));
        set_option(&lib, &mut p, "opt", "fast", None).unwrap();
        set_option(&lib, &mut p, "opt", "sky", None).unwrap();
        assert!(p.options.is_empty(), "reset removes the value, and the mod when it has none left");
    }

    #[test]
    fn keys_check_the_keybind_and_the_name_and_reset() {
        let (_t, lib, mut p) = world();
        set_key(&lib, &mut p, "opt", "toggle", Some("f9")).unwrap();
        assert_eq!(p.keys["opt"]["toggle"], "f9");
        assert!(matches!(set_key(&lib, &mut p, "opt", "nope", Some("f9")), Err(ObsidianError::UnknownKeybind { .. })));
        assert!(set_key(&lib, &mut p, "opt", "toggle", Some("Left Shift")).is_err());
        set_key(&lib, &mut p, "opt", "toggle", None).unwrap();
        assert!(p.keys.is_empty());
    }
}
