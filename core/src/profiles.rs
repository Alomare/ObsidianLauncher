//! Named profiles: `<data>/profiles/<name>.json`, with the active one named in `profiles/active.txt`.
//! A data folder from before profiles (`<data>/profile.json`) becomes the profile "Default".

use std::path::{Path, PathBuf};

use crate::{ObsidianError, Result};

const DIR: &str = "profiles";
const ACTIVE: &str = "active.txt";
const OLD: &str = "profile.json";
pub const DEFAULT: &str = "Default";

fn dir(data: &Path) -> PathBuf {
    data.join(DIR)
}

/// The file of profile `name`.
pub fn path(data: &Path, name: &str) -> PathBuf {
    dir(data).join(format!("{name}.json"))
}

/// Moves `profile.json` into `profiles/Default.json` the first time profiles are used.
fn migrate(data: &Path) -> Result<()> {
    let old = data.join(OLD);
    if dir(data).is_dir() || !old.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(dir(data))?;
    std::fs::rename(&old, path(data, DEFAULT))?;
    std::fs::write(dir(data).join(ACTIVE), DEFAULT)?;
    Ok(())
}

/// Profile names, sorted without regard to case. Profiles named in `active.txt` but never saved
/// (a fresh data folder) are included.
pub fn list(data: &Path) -> Result<Vec<String>> {
    migrate(data)?;
    let mut out = Vec::new();
    if dir(data).is_dir() {
        for e in std::fs::read_dir(dir(data))? {
            let p = e?.path();
            if p.extension().is_some_and(|x| x == "json")
                && let Some(stem) = p.file_stem()
            {
                out.push(stem.to_string_lossy().into_owned());
            }
        }
    }
    let active = active(data)?;
    if !out.iter().any(|n| n == &active) {
        out.push(active);
    }
    out.sort_by_key(|n| n.to_lowercase());
    Ok(out)
}

/// The active profile's name, "Default" when none was ever chosen.
pub fn active(data: &Path) -> Result<String> {
    migrate(data)?;
    match std::fs::read_to_string(dir(data).join(ACTIVE)) {
        Ok(t) if !t.trim().is_empty() => Ok(t.trim().to_string()),
        Ok(_) => Ok(DEFAULT.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DEFAULT.to_string()),
        Err(e) => Err(e.into()),
    }
}

/// Whether two names are one file on Windows, which ignores case beyond ASCII too.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn find(data: &Path, name: &str) -> Result<Option<String>> {
    Ok(list(data)?.into_iter().find(|n| same_name(n, name)))
}

fn set_active(data: &Path, name: &str) -> Result<()> {
    std::fs::create_dir_all(dir(data))?;
    crate::paths::write_atomic(&dir(data).join(ACTIVE), name.as_bytes())?;
    Ok(())
}

/// The profiles that couldn't be read and were set aside.
pub fn damaged(data: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir(data)) else { return Vec::new() };
    let mut out: Vec<String> = entries.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".json.damaged").map(str::to_string)).collect();
    out.sort();
    out
}

pub fn use_profile(data: &Path, name: &str) -> Result<()> {
    let found = find(data, name)?.ok_or_else(|| ObsidianError::ProfileMissing(name.to_string()))?;
    set_active(data, &found)
}

/// A name Windows accepts as a file name, 1 to 40 characters.
fn check_name(name: &str) -> Result<()> {
    const DEVICES: [&str; 4] = ["con", "prn", "aux", "nul"];
    let lower = name.to_lowercase();
    let device = DEVICES.contains(&lower.as_str())
        || ((lower.starts_with("com") || lower.starts_with("lpt")) && lower.len() == 4 && lower.as_bytes()[3].is_ascii_digit() && lower.as_bytes()[3] != b'0');
    let bad = name.is_empty()
        || name.chars().count() > 40
        || name.chars().any(|c| "\\/:*?\"<>|".contains(c) || c.is_control())
        || name.starts_with([' ', '.'])
        || name.ends_with([' ', '.'])
        || device;
    if bad { Err(ObsidianError::ProfileName(name.to_string())) } else { Ok(()) }
}

/// A new profile, with nothing on or a copy of `copy_of`. It does not become active.
pub fn create(data: &Path, name: &str, copy_of: Option<&str>) -> Result<()> {
    check_name(name)?;
    if find(data, name)?.is_some() {
        return Err(ObsidianError::ProfileExists(name.to_string()));
    }
    std::fs::create_dir_all(dir(data))?;
    match copy_of {
        Some(src) => {
            let src = find(data, src)?.ok_or_else(|| ObsidianError::ProfileMissing(src.to_string()))?;
            let from = path(data, &src);
            let text = if from.is_file() { std::fs::read_to_string(from)? } else { "{\"mods\":[]}\n".to_string() };
            std::fs::write(path(data, name), text)?;
        }
        None => crate::starter::blank_profile(data)?.save_named(data, name)?,
    }
    Ok(())
}

pub fn rename(data: &Path, from: &str, to: &str) -> Result<()> {
    check_name(to)?;
    let from = find(data, from)?.ok_or_else(|| ObsidianError::ProfileMissing(from.to_string()))?;
    if let Some(other) = find(data, to)?
        && other != from
    {
        return Err(ObsidianError::ProfileExists(to.to_string()));
    }
    let was_active = active(data)? == from;
    std::fs::create_dir_all(dir(data))?;
    if path(data, &from).is_file() {
        std::fs::rename(path(data, &from), path(data, to))?;
    }
    crate::images::rename(data, crate::images::Owner::Profile, &from, to)?;
    if was_active {
        set_active(data, to)?;
    }
    Ok(())
}

/// Deletes a profile. The last one can't go; deleting the active one makes the first left active.
pub fn delete(data: &Path, name: &str) -> Result<()> {
    let names = list(data)?;
    let found = names.iter().find(|n| n.eq_ignore_ascii_case(name)).cloned().ok_or_else(|| ObsidianError::ProfileMissing(name.to_string()))?;
    if names.len() == 1 {
        return Err(ObsidianError::LastProfile);
    }
    let was_active = active(data)? == found;
    if path(data, &found).is_file() {
        std::fs::remove_file(path(data, &found))?;
    }
    crate::images::remove(data, crate::images::Owner::Profile, &found)?;
    if was_active {
        let next = names.iter().find(|n| **n != found).expect("another profile");
        set_active(data, next)?;
    }
    Ok(())
}

/// Sets or clears the color of a profile. A color is `#rrggbb`.
pub fn set_color(data: &Path, name: &str, color: Option<&str>) -> Result<()> {
    let found = find(data, name)?.ok_or_else(|| ObsidianError::ProfileMissing(name.to_string()))?;
    if let Some(c) = color.filter(|c| !crate::options::is_color(c)) {
        return Err(ObsidianError::BadOptionValue { option: "color".into(), value: c.to_string() });
    }
    let mut p = crate::library::Profile::load_named(data, &found)?;
    p.color = color.map(str::to_ascii_lowercase);
    let file = path(data, &found);
    std::fs::create_dir_all(dir(data))?;
    std::fs::write(file, serde_json::to_string_pretty(&p)? + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Profile;
    use crate::testutil::spaced_tempdir;

    #[test]
    fn old_profile_json_becomes_default() {
        let t = spaced_tempdir();
        let text = "{\"mods\":[{\"id\":\"a.b\",\"enabled\":true}],\"block_public\":true}";
        std::fs::write(t.path().join("profile.json"), text).unwrap();
        assert_eq!(active(t.path()).unwrap(), "Default");
        assert!(!t.path().join("profile.json").exists());
        let p = Profile::load(t.path()).unwrap();
        assert!(p.block_public && p.mods[0].enabled);
    }

    #[test]
    fn profile_color_is_checked_saved_and_cleared() {
        let t = spaced_tempdir();
        create(t.path(), "Blue", None).unwrap();
        set_color(t.path(), "blue", Some("#4A90D9")).unwrap();
        assert_eq!(Profile::load_named(t.path(), "Blue").unwrap().color.as_deref(), Some("#4a90d9"));
        assert!(set_color(t.path(), "Blue", Some("blue")).is_err());
        assert!(set_color(t.path(), "Missing", Some("#000000")).is_err());
        set_color(t.path(), "Blue", None).unwrap();
        assert_eq!(Profile::load_named(t.path(), "Blue").unwrap().color, None);
    }

    #[test]
    fn fresh_data_dir_has_default() {
        let t = spaced_tempdir();
        assert_eq!(list(t.path()).unwrap(), vec!["Default"]);
        assert_eq!(active(t.path()).unwrap(), "Default");
    }

    #[test]
    fn create_copy_rename_delete() {
        let t = spaced_tempdir();
        let p = Profile { block_public: true, ..Default::default() };
        p.save(t.path()).unwrap();
        create(t.path(), "Camera fun", Some("default")).unwrap();
        create(t.path(), "Clean", None).unwrap();
        assert_eq!(list(t.path()).unwrap(), vec!["Camera fun", "Clean", "Default"]);
        use_profile(t.path(), "camera fun").unwrap();
        assert!(Profile::load(t.path()).unwrap().block_public, "copy keeps the settings");
        rename(t.path(), "Camera fun", "Cameras").unwrap();
        assert_eq!(active(t.path()).unwrap(), "Cameras", "renaming the active profile keeps it active");
        use_profile(t.path(), "Clean").unwrap();
        assert!(!Profile::load(t.path()).unwrap().block_public);
        delete(t.path(), "Cameras").unwrap();
        assert_eq!(list(t.path()).unwrap(), vec!["Clean", "Default"]);
    }

    #[test]
    fn delete_refuses_the_last_profile() {
        let t = spaced_tempdir();
        assert!(matches!(delete(t.path(), "Default"), Err(ObsidianError::LastProfile)));
    }

    #[test]
    fn deleting_the_active_profile_switches_to_the_first_left() {
        let t = spaced_tempdir();
        create(t.path(), "B", None).unwrap();
        create(t.path(), "A", None).unwrap();
        delete(t.path(), "Default").unwrap();
        assert_eq!(active(t.path()).unwrap(), "A");
    }

    #[test]
    fn bad_names_are_refused() {
        let t = spaced_tempdir();
        for name in ["", "a/b", "con", "COM3", "x.", " x", "a:b", &"n".repeat(41)] {
            assert!(matches!(create(t.path(), name, None), Err(ObsidianError::ProfileName(_))), "{name:?}");
        }
        create(t.path(), "com0", None).unwrap();
        create(t.path(), &"n".repeat(40), None).unwrap();
    }

    #[test]
    fn existing_name_is_refused() {
        let t = spaced_tempdir();
        assert!(matches!(create(t.path(), "default", None), Err(ObsidianError::ProfileExists(_))));
        create(t.path(), "B", None).unwrap();
        assert!(matches!(rename(t.path(), "B", "DEFAULT"), Err(ObsidianError::ProfileExists(_))));
    }
}
