//! The starter pack, which the player sees as the Obsidian Kit: the mods that come with Obsidian. They
//! are mods like any other, which the player can turn off, move, update from their own repositories
//! and delete, so they go into a data folder once, the first time it is used, and after that only
//! when the player asks (`restore`).

use std::path::Path;

use crate::folders::Folder;
use crate::library::{Library, Profile};
use crate::manifest::{FILE, load_manifest};
use crate::{ObsidianError, Result, loader, profiles};

/// In load order.
pub const MODS: [&str; 3] = ["obsidian.api", "obsidian.trainer", "obsidian.qol"];
/// The mod of the pack that starts on.
const ON: &str = "obsidian.qol";
/// The same id as the Obsidian folder of older versions, which `seed` turns into this one.
const FOLDER: &str = "obsidian";
/// Marks a data folder that has had the pack.
const SEEDED: &str = "starter-pack.txt";

fn folder() -> Folder {
    Folder { id: FOLDER.into(), name: "Obsidian Kit".into(), color: "#4a90d9".into(), collapsed: true, locked: true }
}

/// Copies the shipped mods into the library. A newer copy, from an update, stays.
fn install(lib: &Library, dir: &Path) -> Result<()> {
    for id in MODS {
        let src = dir.join(id);
        if !src.join(FILE).is_file() {
            continue;
        }
        let shipped = load_manifest(&src)?;
        if load_manifest(&lib.mod_dir(id)).map_or(true, |have| have.version <= shipped.version) {
            lib.install(&src)?;
        }
    }
    Ok(())
}

/// Puts the pack first in the load order, in its closed folder. Anything else in the folder leaves it
/// and stays right after.
fn lay_out(p: &mut Profile) {
    match p.folders.iter_mut().find(|f| f.id == FOLDER) {
        Some(f) => *f = folder(),
        None => p.folders.insert(0, folder()),
    }
    for e in &mut p.mods {
        if MODS.contains(&e.id.as_str()) {
            e.folder = Some(FOLDER.into());
        } else if e.folder.as_deref() == Some(FOLDER) {
            e.folder = None;
        }
    }
    p.set_order(&MODS.map(String::from));
    p.tidy_folders();
}

/// Puts the pack back as it came: deleted mods return, and the folder, the order, what is on and
/// every setting go back to the start. The player's other mods stay as they are.
pub fn restore(lib: &Library, p: &mut Profile) -> Result<()> {
    install(lib, &loader::find_bundled_dir().ok_or(ObsidianError::NoStarterPack)?)?;
    p.sync(lib)?;
    lay_out(p);
    for id in MODS {
        p.options.remove(id);
        p.keys.remove(id);
        if let Some(e) = p.mods.iter_mut().find(|m| m.id == id) {
            e.enabled = id == ON;
        }
    }
    // With what it requires (Obsidian Patches needs Obsidian API since 1.4.0).
    if p.mods.iter().any(|m| m.id == ON) {
        p.switch(lib, ON, true)?;
    }
    // A mod of the player's that needs one of these keeps it on.
    let others: Vec<String> = p.mods.iter().filter(|m| m.enabled && !MODS.contains(&m.id.as_str())).map(|m| m.id.clone()).collect();
    for id in others {
        let _ = p.switch(lib, &id, true);
    }
    Ok(())
}

/// Sets the pack up the first time a data folder is used. A data folder from an older version keeps
/// what the player chose; its mods become plain copies and its Obsidian folder this one.
pub fn seed(data: &Path) -> Result<()> {
    let seeded = data.join(SEEDED);
    if seeded.exists() {
        return Ok(());
    }
    let Some(dir) = loader::find_bundled_dir() else { return Ok(()) };
    let lib = Library::open(data)?;
    let active = profiles::active(data)?;
    if profiles::path(data, &active).is_file() {
        install(&lib, &dir)?;
        for name in profiles::list(data)? {
            let mut p = Profile::load_named(data, &name)?;
            if let Some(f) = p.folders.iter_mut().find(|f| f.id == FOLDER) {
                *f = folder();
                p.save_named(data, &name)?;
            }
        }
    } else {
        let mut p = Profile::default();
        restore(&lib, &mut p)?;
        p.save(data)?;
    }
    std::fs::write(seeded, "The starter pack was set up here. Restore it from Settings.\n")?;
    Ok(())
}

/// A new empty profile: nothing on, and the pack in its folder when it's in the library.
pub fn blank_profile(data: &Path) -> Result<Profile> {
    let mut p = Profile::default();
    p.sync(&Library::open(data)?)?;
    if p.mods.iter().any(|m| MODS.contains(&m.id.as_str())) {
        lay_out(&mut p);
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::effective_values;
    use crate::testutil::{put, spaced_tempdir};
    use serde_json::{Value, json};

    fn order(p: &Profile) -> Vec<&str> {
        p.mods.iter().map(|m| m.id.as_str()).collect()
    }
    fn on(p: &Profile) -> Vec<&str> {
        p.mods.iter().filter(|m| m.enabled).map(|m| m.id.as_str()).collect()
    }
    fn stage(root: &Path, id: &str, version: &str, needs: &str) -> std::path::PathBuf {
        let d = root.join(format!("stage-{id}-{version}"));
        put(&d, FILE, format!(r#"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}},"requires":[{needs}]}}"#).as_bytes());
        d
    }
    fn folders_ok(p: &Profile) {
        assert_eq!(p.folders, [folder()]);
        assert_eq!((p.folders[0].name.as_str(), p.folders[0].color.as_str(), p.folders[0].collapsed, p.folders[0].locked), ("Obsidian Kit", "#4a90d9", true, true));
        assert!(p.mods.iter().take(3).all(|m| m.folder.as_deref() == Some(FOLDER)));
    }
    /// The values the game gets for Obsidian Patches.
    fn qol_values(lib: &Library, p: &Profile) -> Vec<(String, Value)> {
        let specs = load_manifest(&lib.mod_dir(ON)).unwrap().option_specs().unwrap();
        effective_values(&specs, p.options.get(ON)).into_iter().collect()
    }

    #[test]
    fn a_new_data_folder_starts_with_the_pack_as_specified() {
        let t = spaced_tempdir();
        seed(t.path()).unwrap();
        let lib = Library::open(t.path()).unwrap();
        let p = Profile::load(t.path()).unwrap();
        assert_eq!(order(&p), MODS);
        assert_eq!(on(&p), ["obsidian.api", ON], "Obsidian Patches, with the API it requires");
        folders_ok(&p);
        let values = qol_values(&lib, &p);
        let toggled: Vec<&str> = values.iter().filter(|(_, v)| *v == json!(true)).map(|(k, _)| k.as_str()).collect();
        assert_eq!(toggled, ["dive_sprint_fix", "recoil_fix", "skip_intro", "skip_login", "skip_planet_scanning", "solo_pause", "widescreen"], "the manifest's defaults");
        assert!(values.contains(&("expedite_mission_end".to_string(), json!("faster"))));
        for id in MODS {
            assert!(crate::updates::source_of(&load_manifest(&lib.mod_dir(id)).unwrap()).is_some(), "{id} updates from its repository");
        }
    }

    #[test]
    fn the_pack_is_set_up_once_so_the_players_changes_last() {
        let t = spaced_tempdir();
        seed(t.path()).unwrap();
        let lib = Library::open(t.path()).unwrap();
        lib.remove("obsidian.trainer").unwrap();
        lib.install(&stage(t.path(), "obsidian.api", "9.0.0", "")).unwrap();
        let mut p = Profile::load(t.path()).unwrap();
        p.sync(&lib).unwrap();
        assert!(p.delete_folder(FOLDER).is_empty() && p.folders.len() == 1, "the folder starts locked");
        p.edit_folder(FOLDER, None, None, None, Some(false)).unwrap();
        assert_eq!(p.delete_folder(FOLDER), ["obsidian.api", "obsidian.qol"], "and unlocks like any other");
        p.set_enabled(ON, false).unwrap();
        p.set_enabled("obsidian.api", false).unwrap();
        p.save(t.path()).unwrap();

        seed(t.path()).unwrap();
        let mut q = Profile::load(t.path()).unwrap();
        q.sync(&lib).unwrap();
        assert!(!lib.mod_dir("obsidian.trainer").exists(), "a deleted mod stays deleted");
        assert_eq!(load_manifest(&lib.mod_dir("obsidian.api")).unwrap().version.to_string(), "9.0.0", "an update stays");
        assert!(q.folders.is_empty() && on(&q).is_empty());
    }

    #[test]
    fn restore_brings_everything_back_and_leaves_other_mods_alone() {
        let t = spaced_tempdir();
        seed(t.path()).unwrap();
        let lib = Library::open(t.path()).unwrap();
        lib.install(&stage(t.path(), "my.mod", "1.0.0", "")).unwrap();
        lib.install(&stage(t.path(), "my.other", "1.0.0", "")).unwrap();
        lib.remove("obsidian.qol").unwrap();
        let mut p = Profile::load(t.path()).unwrap();
        p.sync(&lib).unwrap();
        p.set_order(&["my.mod".into(), "my.other".into()]);
        p.set_enabled("my.mod", true).unwrap();
        p.switch(&lib, "obsidian.trainer", true).unwrap();
        p.edit_folder(FOLDER, Some("Mine"), Some("#ffffff"), Some(false), Some(false)).unwrap();
        p.arrange(&[], &[("my.other".into(), Some(FOLDER.into()))]);
        p.options.insert(ON.into(), [("skip_intro".to_string(), json!(true))].into());
        p.options.insert("my.mod".into(), [("speed".to_string(), json!(3))].into());
        p.keys.insert("obsidian.trainer".into(), [("open".to_string(), "f9".to_string())].into());

        restore(&lib, &mut p).unwrap();
        assert_eq!(order(&p), ["obsidian.api", "obsidian.trainer", "obsidian.qol", "my.mod", "my.other"]);
        assert_eq!(on(&p), ["obsidian.api", ON, "my.mod"]);
        folders_ok(&p);
        assert_eq!(p.mods[4].folder, None, "another mod leaves the folder");
        assert!(lib.mod_dir(ON).is_dir(), "a deleted mod returns");
        assert_eq!(p.options.keys().collect::<Vec<_>>(), ["my.mod"], "only the pack's settings go");
        assert!(p.keys.is_empty());
    }

    #[test]
    fn restore_keeps_a_newer_copy_and_repairs_an_equal_one() {
        let t = spaced_tempdir();
        seed(t.path()).unwrap();
        let lib = Library::open(t.path()).unwrap();
        lib.install(&stage(t.path(), "obsidian.api", "9.0.0", "")).unwrap();
        let qol = load_manifest(&lib.mod_dir(ON)).unwrap().version.to_string();
        lib.install(&stage(t.path(), ON, &qol, "")).unwrap();
        assert!(!lib.mod_dir(ON).join("main.lua").exists());

        let mut p = Profile::load(t.path()).unwrap();
        restore(&lib, &mut p).unwrap();
        assert_eq!(load_manifest(&lib.mod_dir("obsidian.api")).unwrap().version.to_string(), "9.0.0");
        assert!(lib.mod_dir(ON).join("main.lua").is_file());
    }

    #[test]
    fn restore_leaves_on_what_a_players_mod_needs() {
        let t = spaced_tempdir();
        seed(t.path()).unwrap();
        let lib = Library::open(t.path()).unwrap();
        lib.install(&stage(t.path(), "my.mod", "1.0.0", r#""obsidian.trainer""#)).unwrap();
        let mut p = Profile::load(t.path()).unwrap();
        p.sync(&lib).unwrap();
        p.switch(&lib, "my.mod", true).unwrap();
        restore(&lib, &mut p).unwrap();
        assert_eq!(on(&p), [MODS[0], MODS[1], MODS[2], "my.mod"]);
    }

    #[test]
    fn an_older_data_folder_keeps_the_players_choices() {
        let t = spaced_tempdir();
        let lib = Library::open(t.path()).unwrap();
        for id in MODS {
            let d = t.path().join(format!("old-{id}"));
            put(&d, FILE, format!(r#"{{"obsidian":1,"id":"{id}","version":"0.1.0","name":{{"en":"{id}"}},"bundled":true}}"#).as_bytes());
            lib.install(&d).unwrap();
        }
        let old = r##"{"mods":[{"id":"obsidian.api","enabled":true,"folder":"obsidian"},{"id":"obsidian.qol","enabled":false,"folder":"obsidian"},{"id":"obsidian.trainer","enabled":true,"folder":"obsidian"}],
            "options":{"obsidian.qol":{"skip_intro":true}},"folders":[{"id":"obsidian","name":"Obsidian","color":"#f0b429","collapsed":false,"locked":true}]}"##;
        put(&t.path().join("profiles"), "Default.json", old.as_bytes());
        put(&t.path().join("profiles"), "Second.json", old.as_bytes());

        seed(t.path()).unwrap();
        for name in ["Default", "Second"] {
            let p = Profile::load_named(t.path(), name).unwrap();
            assert_eq!(p.folders, [folder()], "{name}");
            assert_eq!(on(&p), ["obsidian.api", "obsidian.trainer"]);
            assert_eq!(p.options[ON]["skip_intro"], json!(true));
        }
        for id in MODS {
            assert!(!load_manifest(&lib.mod_dir(id)).unwrap().extra.contains_key("bundled"), "{id}");
        }
    }

    #[test]
    fn a_deploy_says_whether_a_needed_mod_is_off_or_gone() {
        let t = spaced_tempdir();
        seed(t.path()).unwrap();
        let lib = Library::open(t.path()).unwrap();
        let mut p = Profile::load(t.path()).unwrap();
        p.switch(&lib, "obsidian.trainer", true).unwrap();
        p.set_enabled("obsidian.api", false).unwrap();
        let off = crate::compose::compose(&lib, &p).err().unwrap();
        assert_eq!(off.to_string(), "obsidian.trainer needs obsidian.api, which is not switched on");
        lib.remove("obsidian.api").unwrap();
        p.sync(&lib).unwrap();
        let gone = crate::compose::compose(&lib, &p).err().unwrap();
        assert_eq!(gone.to_string(), "obsidian.trainer needs obsidian.api, which is not installed");
    }

    #[test]
    fn a_blank_profile_has_the_folder_and_nothing_on() {
        let t = spaced_tempdir();
        assert!(blank_profile(t.path()).unwrap().folders.is_empty(), "no pack in the library, no folder");
        seed(t.path()).unwrap();
        let p = blank_profile(t.path()).unwrap();
        assert_eq!(order(&p), MODS);
        assert!(on(&p).is_empty());
        folders_ok(&p);
    }
}
