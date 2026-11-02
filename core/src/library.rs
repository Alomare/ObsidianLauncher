//! The mod library (`<data>/mods/<id>/`) and the load-order profile (the active one in `<data>/profiles/`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::manifest::{FILE, Manifest, load_manifest};
use crate::paths::{rename_retrying, write_atomic};
use crate::{ObsidianError, Result};

pub struct Library {
    root: PathBuf,
}

/// Where a legacy mod came from, kept as `import.json` in the mod folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportInfo {
    /// RFC 3339.
    pub date: String,
    /// The folder or file name the mod was imported from.
    pub file: String,
}

pub const IMPORT_INFO: &str = "import.json";

/// The name of the old copy of a mod while a new one goes in, as an extension of its folder.
const OLD: &str = "obsidian-old";
/// Added to the file name of a profile that can't be read.
const DAMAGED: &str = ".damaged";

/// A mod folder whose manifest can't be read. It stays out of the game until it's fixed or removed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BrokenMod {
    pub folder: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstallOutcome {
    Added,
    Replaced { old_version: semver::Version },
}

/// Copies a folder tree.
pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let target = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &target)?;
        } else {
            std::fs::copy(e.path(), target)?;
        }
    }
    Ok(())
}

impl Library {
    pub fn open(data_dir: &Path) -> Result<Library> {
        let root = data_dir.join("mods");
        std::fs::create_dir_all(&root)?;
        Ok(Library { root })
    }
    pub fn mod_dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }
    /// The data folder the library lives in.
    pub fn data_dir(&self) -> &Path {
        self.root.parent().unwrap_or(&self.root)
    }
    /// Copies a staged folder holding `obsidian.json` into `mods/<id>`, replacing an older copy.
    pub fn install(&self, staged: &Path) -> Result<InstallOutcome> {
        let m = load_manifest(staged)?;
        let target = self.mod_dir(&m.id);
        let tmp = self.root.join(format!("{}.obsidian-tmp", m.id));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        copy_dir(staged, &tmp)?;
        let outcome = match load_manifest(&target) {
            Ok(old) => InstallOutcome::Replaced { old_version: old.version },
            Err(_) => InstallOutcome::Added,
        };
        // The old copy steps aside rather than going first, so a failed swap loses nothing.
        let aside = self.root.join(format!("{}.{OLD}", m.id));
        if aside.exists() {
            std::fs::remove_dir_all(&aside)?;
        }
        if target.exists() {
            rename_retrying(&target, &aside)?;
        }
        if let Err(e) = rename_retrying(&tmp, &target) {
            if aside.exists() {
                let _ = rename_retrying(&aside, &target);
            }
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(e.into());
        }
        let _ = std::fs::remove_dir_all(&aside);
        crate::manifest::forget(&target);
        Ok(outcome)
    }
    pub fn import_info(&self, id: &str) -> Option<ImportInfo> {
        let text = std::fs::read_to_string(self.mod_dir(id).join(IMPORT_INFO)).ok()?;
        serde_json::from_str(&text).ok()
    }
    /// Deletes a mod from the library. Profiles drop it at their next sync.
    pub fn remove(&self, id: &str) -> Result<()> {
        let dir = self.mod_dir(id);
        if !dir.join(FILE).is_file() {
            return Err(ObsidianError::ModMissing(id.to_string()));
        }
        crate::manifest::forget(&dir);
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<Manifest>> {
        Ok(self.scan()?.0)
    }
    /// `list` and `broken` from one pass over the folders, for a caller that wants both.
    pub fn list_and_broken(&self) -> Result<(Vec<Manifest>, Vec<BrokenMod>)> {
        self.scan()
    }
    /// The folders that look like mods but can't be read.
    pub fn broken(&self) -> Result<Vec<BrokenMod>> {
        Ok(self.scan()?.1)
    }
    /// The mods, and the folders that can't be read: one bad manifest must not take the whole app
    /// down, Purge included. A swap cut short by a crash is put right on the way.
    fn scan(&self) -> Result<(Vec<Manifest>, Vec<BrokenMod>)> {
        let (mut good, mut broken) = (Vec::new(), Vec::new());
        for e in std::fs::read_dir(&self.root)? {
            let e = e?;
            let mut p = e.path();
            if !e.file_type()?.is_dir() || p.extension().is_some_and(|x| x == "obsidian-tmp") {
                continue;
            }
            if p.extension().is_some_and(|x| x == OLD) {
                let target = p.with_extension("");
                if target.exists() {
                    let _ = std::fs::remove_dir_all(&p);
                    continue;
                }
                rename_retrying(&p, &target)?;
                p = target;
            }
            match load_manifest(&p) {
                Ok(m) => good.push(m),
                // A folder without a manifest is not a mod.
                Err(ObsidianError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => broken.push(BrokenMod { folder: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), reason: e.to_string() }),
            }
        }
        good.sort_by(|a: &Manifest, b| a.id.cmp(&b.id));
        good.dedup_by(|a, b| a.id == b.id);
        broken.sort_by(|a, b| a.folder.cmp(&b.folder));
        Ok((good, broken))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub id: String,
    pub enabled: bool,
    /// The folder the mod sits in, by folder id (`crate::folders`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// Load order: the list order, where the last enabled mod wins a conflict.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Profile {
    pub mods: Vec<ProfileEntry>,
    /// "Allow session mods from other Obsidian users".
    #[serde(default)]
    pub consent: bool,
    /// No Steam achievement unlocks, and no achievement progress kept, while this profile plays.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub block_achievements: bool,
    /// Nothing about the account's progression reaches the save while this profile plays: XP, samples,
    /// unlocks, upgrades, stats and achievement progress stay as they were at launch.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub freeze_progress: bool,
    /// Option values per mod id, as set by the player.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    /// Keys per mod id and keybind id, as set by the player.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub keys: BTreeMap<String, BTreeMap<String, String>>,
    /// Folders the player sorts mods into. They change nothing in the game.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<crate::folders::Folder>,
    /// The color the app shows for this profile (`#rrggbb`); the default gold when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Profile {
    /// The active profile.
    pub fn load(data_dir: &Path) -> Result<Profile> {
        Profile::load_named(data_dir, &crate::profiles::active(data_dir)?)
    }
    /// A profile that can't be read is set aside and starts over: it must not lock the app. The
    /// file stays for the player, and `profiles::damaged` lists it.
    pub fn load_named(data_dir: &Path, name: &str) -> Result<Profile> {
        let path = crate::profiles::path(data_dir, name);
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(p) => Ok(p),
                Err(_) => {
                    let mut aside = path.clone().into_os_string();
                    aside.push(DAMAGED);
                    rename_retrying(&path, Path::new(&aside))?;
                    Ok(Profile::default())
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Profile::default()),
            Err(e) => Err(e.into()),
        }
    }
    /// Saves as the active profile.
    pub fn save(&self, data_dir: &Path) -> Result<()> {
        self.save_named(data_dir, &crate::profiles::active(data_dir)?)
    }
    pub fn save_named(&self, data_dir: &Path, name: &str) -> Result<()> {
        let path = crate::profiles::path(data_dir, name);
        std::fs::create_dir_all(path.parent().expect("profiles folder"))?;
        write_atomic(&path, (serde_json::to_string_pretty(self)? + "\n").as_bytes())?;
        Ok(())
    }
    /// Puts `ids` first, in that order; mods not named keep their order after them.
    pub fn set_order(&mut self, ids: &[String]) {
        let mut first: Vec<ProfileEntry> = ids.iter().filter_map(|id| self.mods.iter().find(|m| &m.id == id).cloned()).collect();
        first.extend(self.mods.iter().filter(|m| !ids.contains(&m.id)).cloned());
        self.mods = first;
    }
    pub fn set_enabled(&mut self, id: &str, on: bool) -> Result<()> {
        let entry = self.mods.iter_mut().find(|m| m.id == id).ok_or_else(|| ObsidianError::ModMissing(id.to_string()))?;
        entry.enabled = on;
        Ok(())
    }
    /// Switches a mod on with the mods it requires, or off with the mods that require it. Returns
    /// the other mods that changed.
    pub fn switch(&mut self, lib: &Library, id: &str, on: bool) -> Result<Vec<String>> {
        let manifests = lib.list()?;
        let mut todo = vec![id.to_string()];
        let mut changed = Vec::new();
        while let Some(next) = todo.pop() {
            let entry = self.mods.iter_mut().find(|m| m.id == next).ok_or_else(|| ObsidianError::ModMissing(next.clone()))?;
            if entry.enabled == on && next != id {
                continue;
            }
            entry.enabled = on;
            if next != id {
                changed.push(next.clone());
            }
            if on {
                let needs = manifests.iter().find(|m| m.id == next).map(|m| m.requires.clone()).unwrap_or_default();
                if let Some(gone) = needs.iter().find(|r| !self.mods.iter().any(|m| m.id == **r)) {
                    return Err(ObsidianError::RequirementNotInstalled { mod_id: next, requires: gone.clone() });
                }
                todo.extend(needs);
            } else {
                todo.extend(manifests.iter().filter(|m| m.requires.contains(&next)).map(|m| m.id.clone()));
            }
        }
        Ok(changed)
    }
    /// Adds newly installed mods as disabled and drops entries whose mod is gone. A new mod goes last,
    /// unless a load hint says otherwise (`place_new`).
    pub fn sync(&mut self, lib: &Library) -> Result<()> {
        let installed = lib.list()?;
        self.sync_with(lib, &installed)
    }

    /// `sync` with the library's mods already read.
    pub fn sync_with(&mut self, lib: &Library, installed: &[Manifest]) -> Result<()> {
        // A mod that can't be read keeps its place, so fixing it doesn't lose the load order.
        self.mods.retain(|m| installed.iter().any(|i| i.id == m.id) || lib.mod_dir(&m.id).join(FILE).is_file());
        for m in installed {
            if !self.mods.iter().any(|e| e.id == m.id) {
                let at = self.place_new(m, installed);
                self.mods.insert(at, ProfileEntry { id: m.id.clone(), enabled: false, folder: None });
            }
        }
        self.tidy_folders();
        Ok(())
    }

    /// Where a new mod goes: before the first mod that asks to load after it, unless it asks to load
    /// after one further down. Only a mod's arrival uses this: the order the player set stays.
    fn place_new(&self, m: &Manifest, installed: &[Manifest]) -> usize {
        let wants_after = |e: &ProfileEntry| {
            installed.iter().find(|o| o.id == e.id).is_some_and(|o| o.load_after.contains(&m.id) || (m.load.is_none() && o.load.is_some()))
        };
        let mut before = self.mods.iter().position(wants_after).unwrap_or(self.mods.len());
        // A folder's mods stay together, so the place before one inside a folder is before the folder.
        while before > 0 && before < self.mods.len() && self.mods[before].folder.is_some() && self.mods[before - 1].folder == self.mods[before].folder {
            before -= 1;
        }
        let after = self.mods.iter().rposition(|e| m.load_after.contains(&e.id)).map_or(0, |i| i + 1);
        before.max(after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    fn staged(root: &Path, id: &str, version: &str) -> PathBuf {
        let d = root.join(format!("stage-{id}-{version}"));
        let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}}}}"#);
        put(&d, "obsidian.json", json.as_bytes());
        d
    }

    fn hinted(root: &Path, id: &str, hint: &str) -> PathBuf {
        let d = root.join(format!("stage-{id}"));
        let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},{hint}}}"#);
        put(&d, "obsidian.json", json.as_bytes());
        d
    }

    fn order_of(p: &Profile) -> Vec<&str> {
        p.mods.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn a_mod_that_asks_to_load_last_stays_after_new_ones() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&staged(t.path(), "plain", "1.0.0")).unwrap();
        lib.install(&hinted(t.path(), "zlast", r#""load":"last""#)).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        lib.install(&staged(t.path(), "newer", "1.0.0")).unwrap();
        p.sync(&lib).unwrap();
        assert_eq!(order_of(&p).last(), Some(&"zlast"));
        assert_eq!(order_of(&p).len(), 3);
    }

    #[test]
    fn load_after_places_a_mod_on_arrival_either_way_round() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&staged(t.path(), "first", "1.0.0")).unwrap();
        lib.install(&staged(t.path(), "second", "1.0.0")).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        p.set_order(&["second".to_string(), "first".to_string()]);
        // Asks to follow the one that's first: it goes right where it arrives, after that one.
        lib.install(&hinted(t.path(), "after-first", r#""load_after":["first"]"#)).unwrap();
        p.sync(&lib).unwrap();
        assert_eq!(order_of(&p), ["second", "first", "after-first"]);
        // Named by a mod already there: the new one goes before it.
        lib.install(&hinted(t.path(), "needs-x", r#""load_after":["x-mod"]"#)).unwrap();
        p.sync(&lib).unwrap();
        lib.install(&staged(t.path(), "x-mod", "1.0.0")).unwrap();
        p.sync(&lib).unwrap();
        let order = order_of(&p);
        let at = |id: &str| order.iter().position(|o| *o == id).unwrap();
        assert!(at("x-mod") < at("needs-x"), "{order:?}");
        // The player's order stays after that.
        let flipped: Vec<String> = order.iter().rev().map(|s| s.to_string()).collect();
        p.set_order(&flipped);
        p.sync(&lib).unwrap();
        assert_eq!(p.mods.iter().map(|m| m.id.clone()).collect::<Vec<_>>(), flipped);
    }

    #[test]
    fn a_new_mod_goes_before_the_folder_that_holds_a_last_mod() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&staged(t.path(), "boxed", "1.0.0")).unwrap();
        lib.install(&hinted(t.path(), "zlast", r#""load":"last""#)).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        p.folders.push(crate::folders::Folder { id: "f".into(), name: "F".into(), color: "#fff".into(), collapsed: false, locked: false });
        for e in &mut p.mods {
            e.folder = Some("f".into());
        }
        lib.install(&staged(t.path(), "newer", "1.0.0")).unwrap();
        p.sync(&lib).unwrap();
        assert_eq!(order_of(&p), ["newer", "boxed", "zlast"]);
    }

    #[test]
    fn load_hints_are_checked() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        assert!(lib.install(&hinted(t.path(), "bad-load", r#""load":"first""#)).is_err() || load_manifest(&lib.mod_dir("bad-load")).is_err());
        assert!(load_manifest(&hinted(t.path(), "bad-after", r#""load_after":["bad-after"]"#)).is_err());
    }

    fn needing(root: &Path, id: &str, needs: &str) -> PathBuf {
        let d = root.join(format!("stage-{id}"));
        let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},"requires":["{needs}"]}}"#);
        put(&d, "obsidian.json", json.as_bytes());
        d
    }

    #[test]
    fn switch_carries_requirements_along() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&staged(t.path(), "base", "1.0.0")).unwrap();
        lib.install(&needing(t.path(), "middle", "base")).unwrap();
        lib.install(&needing(t.path(), "top", "middle")).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        let on = |p: &Profile, id: &str| p.mods.iter().find(|m| m.id == id).unwrap().enabled;

        let mut changed = p.switch(&lib, "top", true).unwrap();
        changed.sort();
        assert_eq!(changed, ["base", "middle"]);
        assert!(on(&p, "base") && on(&p, "middle") && on(&p, "top"));

        let changed = p.switch(&lib, "middle", false).unwrap();
        assert_eq!(changed, ["top"], "mods that need it go off too, and the base stays on");
        assert!(on(&p, "base") && !on(&p, "middle") && !on(&p, "top"));
    }

    #[test]
    fn switch_names_a_requirement_that_is_not_installed() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&needing(t.path(), "top", "gone")).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        let e = p.switch(&lib, "top", true).unwrap_err();
        assert_eq!(e.to_string(), "top needs gone, which is not installed");
    }

    #[test]
    fn a_mod_that_cant_be_read_is_set_aside_and_keeps_its_place() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        lib.install(&staged(t.path(), "a", "1.0.0")).unwrap();
        lib.install(&staged(t.path(), "b", "1.0.0")).unwrap();
        let mut p = Profile::default();
        p.sync(&lib).unwrap();
        p.set_order(&["b".into(), "a".into()]);
        put(&lib.mod_dir("b"), "obsidian.json", b"{ not json");
        crate::manifest::forget(&lib.mod_dir("b"));

        assert_eq!(lib.list().unwrap().iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["a"]);
        assert_eq!(lib.broken().unwrap()[0].folder, "b");
        p.sync(&lib).unwrap();
        assert_eq!(p.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["b", "a"], "still in the load order");
        lib.remove("b").unwrap();
        assert!(lib.broken().unwrap().is_empty(), "and it can be removed");
    }

    #[test]
    fn a_failed_swap_keeps_the_old_copy() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        lib.install(&staged(t.path(), "a", "1.0.0")).unwrap();
        // A crash after the old copy stepped aside and before the new one landed.
        std::fs::rename(lib.mod_dir("a"), data.join("mods").join("a.obsidian-old")).unwrap();
        assert_eq!(lib.list().unwrap()[0].version, semver::Version::new(1, 0, 0), "the next look puts it back");
        assert!(lib.mod_dir("a").is_dir());
        // A crash after the swap, before the old copy went.
        put(&data.join("mods").join("a.obsidian-old"), "obsidian.json", b"old");
        assert_eq!(lib.list().unwrap().len(), 1);
        assert!(!data.join("mods").join("a.obsidian-old").exists());
    }

    #[test]
    fn a_damaged_profile_starts_over_and_stays_for_the_player() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let path = crate::profiles::path(&data, "Default");
        put(path.parent().unwrap(), "Default.json", b"{ half a fi");
        assert_eq!(Profile::load(&data).unwrap(), Profile::default());
        assert!(!path.exists());
        assert_eq!(crate::profiles::damaged(&data), ["Default"]);
        Profile::default().save(&data).unwrap();
        assert_eq!(Profile::load(&data).unwrap(), Profile::default());
    }

    #[test]
    fn remove_deletes_the_folder() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        lib.install(&staged(t.path(), "a", "1.0.0")).unwrap();
        lib.remove("a").unwrap();
        assert!(lib.list().unwrap().is_empty());
        assert!(!lib.mod_dir("a").exists());
        assert!(matches!(lib.remove("a"), Err(ObsidianError::ModMissing(_))));
    }

    #[test]
    fn install_then_list_and_replace() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        assert_eq!(lib.install(&staged(t.path(), "a", "1.0.0")).unwrap(), InstallOutcome::Added);
        assert_eq!(lib.list().unwrap()[0].id, "a");
        let again = lib.install(&staged(t.path(), "a", "2.0.0")).unwrap();
        assert_eq!(again, InstallOutcome::Replaced { old_version: semver::Version::new(1, 0, 0) });
        assert_eq!(lib.list().unwrap()[0].version, semver::Version::new(2, 0, 0));
    }

    #[test]
    fn profile_syncs_orders_and_persists() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        for id in ["a", "b", "c"] {
            lib.install(&staged(t.path(), id, "1.0.0")).unwrap();
        }
        let mut p = Profile::load(&data).unwrap();
        p.sync(&lib).unwrap();
        assert_eq!(p.mods.iter().map(|m| (m.id.as_str(), m.enabled)).collect::<Vec<_>>(), [("a", false), ("b", false), ("c", false)]);
        p.set_order(&["c".into(), "a".into()]);
        p.set_enabled("a", true).unwrap();
        assert_eq!(p.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["c", "a", "b"]);
        p.save(&data).unwrap();
        assert_eq!(Profile::load(&data).unwrap(), p);
        assert!(p.set_enabled("zzz", true).is_err());
    }

    #[test]
    fn data_dir_prefers_the_explicit_path() {
        let t = spaced_tempdir();
        assert_eq!(crate::paths::data_dir(Some(t.path())), t.path());
    }
}
