//! Changing a mod's option values and keys in a profile, as the CLI and the app both do, and
//! writing them to the settings file the loader reads at launch.

use std::path::Path;

use serde_json::Value;

use crate::library::{Library, Profile};
use crate::manifest::Manifest;
use crate::options::{effective_values, parse_keybinds, validate};
use crate::{ObsidianError, Result};

/// When a changed option reaches the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applies {
    /// Through the settings file, at the next launch.
    NextLaunch,
    /// It picks resources or is part of a session mod's set, so it needs a deploy.
    AfterDeploy,
}

/// The manifest of one mod, read from its folder alone rather than the whole library.
fn manifest(lib: &Library, id: &str) -> Result<Manifest> {
    let missing = || ObsidianError::ModMissing(id.to_string());
    if !crate::manifest::is_valid_id(id) {
        return Err(missing());
    }
    crate::manifest::load_manifest(&lib.mod_dir(id)).ok().filter(|m| m.id == id).ok_or_else(missing)
}

/// Sets an option to `value`, or back to the default when `value` is `None`.
pub fn set_option(lib: &Library, profile: &mut Profile, id: &str, option: &str, value: Option<&Value>) -> Result<Applies> {
    set_option_of(&manifest(lib, id)?, profile, option, value)
}

/// `set_option` with the mod's manifest already read.
pub fn set_option_of(m: &Manifest, profile: &mut Profile, option: &str, value: Option<&Value>) -> Result<Applies> {
    let id = m.id.as_str();
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
    // With the values after the change: a part switched on or off changes what the rest counts as.
    let values = effective_values(&specs, profile.options.get(id));
    Ok(if m.option_needs_deploy(spec, &values) { Applies::AfterDeploy } else { Applies::NextLaunch })
}

type BindMap = std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>;

/// The keybind `keybind` of mod `id`, checked to exist, then `value` set in `map` (a field of the
/// profile), or taken out when `None`, an emptied mod entry with it. `ok` checks a value.
fn set_bind(lib: &Library, map: &mut BindMap, id: &str, keybind: &str, value: Option<&str>, ok: impl Fn(&str) -> bool) -> Result<()> {
    let m = manifest(lib, id)?;
    if !parse_keybinds(&m.keybinds)?.iter().any(|k| k.id == keybind) {
        return Err(ObsidianError::UnknownKeybind { mod_id: id.into(), keybind: keybind.into() });
    }
    match value {
        Some(v) if !ok(v) => return Err(ObsidianError::BadOptionValue { option: keybind.into(), value: v.into() }),
        Some(v) => {
            map.entry(id.to_string()).or_default().insert(keybind.to_string(), v.to_string());
        }
        None => {
            if let Some(set) = map.get_mut(id) {
                set.remove(keybind);
                if set.is_empty() {
                    map.remove(id);
                }
            }
        }
    }
    Ok(())
}

/// Binds a keybind to `key`, or back to the mod's default when `key` is `None`.
pub fn set_key(lib: &Library, profile: &mut Profile, id: &str, keybind: &str, key: Option<&str>) -> Result<()> {
    set_bind(lib, &mut profile.keys, id, keybind, key, crate::options::is_key_name)
}

/// Sets how a keybind fires (`crate::options::INPUT_MODES`), or back to press when `mode` is `None`.
/// Press is the default, so it is kept out of the profile.
pub fn set_mode(lib: &Library, profile: &mut Profile, id: &str, keybind: &str, mode: Option<&str>) -> Result<()> {
    if mode.is_some_and(|x| !crate::options::INPUT_MODES.contains(&x)) {
        return set_bind(lib, &mut profile.modes, id, keybind, mode, |_| false);
    }
    let mode = mode.filter(|x| *x != crate::options::INPUT_MODES[0]);
    set_bind(lib, &mut profile.modes, id, keybind, mode, |_| true)
}

/// Sets the controller input of a keybind (`crate::options::is_pad_combo`), or none when `combo` is `None`:
/// then the mod's own controller input, if it has one, stays.
pub fn set_pad(lib: &Library, profile: &mut Profile, id: &str, keybind: &str, combo: Option<&str>) -> Result<()> {
    set_bind(lib, &mut profile.pads, id, keybind, combo, crate::options::is_pad_combo)
}

/// Writes the profile's consent, guardrails, option values and keys where the loader reads them, and
/// notes them in the deploy record: they are in the game now, so they leave no change waiting.
pub fn write_settings(lib: &Library, profile: &Profile, saves_dir: &Path) -> Result<()> {
    let settings = crate::loader::settings_for(lib, profile)?;
    crate::settings::write(&crate::settings::game_side_dir(saves_dir), &settings)?;
    crate::deploy::note_settings(lib.data_dir(), &settings)
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

    #[test]
    fn modes_check_the_name_keep_press_out_and_reach_the_loader() {
        let (_t, lib, mut p) = world();
        set_mode(&lib, &mut p, "opt", "toggle", Some("long_press")).unwrap();
        assert_eq!(p.modes["opt"]["toggle"], "long_press");
        p.mods.iter_mut().for_each(|m| m.enabled = true);
        let s = crate::loader::settings_for(&lib, &p).unwrap();
        assert_eq!(s.modes, vec![("opt".to_string(), "toggle".to_string(), "long_press".to_string())]);
        assert!(crate::settings::render(&s).contains("mode\topt\ttoggle\tlong_press\n"));
        assert!(matches!(set_mode(&lib, &mut p, "opt", "nope", Some("tap")), Err(ObsidianError::UnknownKeybind { .. })));
        assert!(set_mode(&lib, &mut p, "opt", "toggle", Some("mash")).is_err());
        set_mode(&lib, &mut p, "opt", "toggle", Some("press")).unwrap();
        assert!(p.modes.is_empty(), "press is the default, kept out of the profile");
    }

    #[test]
    fn pads_check_the_buttons_and_reach_the_loader() {
        let (_t, lib, mut p) = world();
        set_pad(&lib, &mut p, "opt", "toggle", Some("left_shoulder+d_up")).unwrap();
        assert_eq!(p.pads["opt"]["toggle"], "left_shoulder+d_up");
        p.mods.iter_mut().for_each(|m| m.enabled = true);
        let s = crate::loader::settings_for(&lib, &p).unwrap();
        assert!(crate::settings::render(&s).contains("pad\topt\ttoggle\tleft_shoulder+d_up\n"));
        assert!(matches!(set_pad(&lib, &mut p, "opt", "nope", Some("a")), Err(ObsidianError::UnknownKeybind { .. })));
        for bad in ["", "a+a", "a+b+x+y+back", "lb", "A"] {
            assert!(set_pad(&lib, &mut p, "opt", "toggle", Some(bad)).is_err(), "{bad}");
        }
        set_pad(&lib, &mut p, "opt", "toggle", None).unwrap();
        assert!(p.pads.is_empty());
    }
}
