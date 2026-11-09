//! A profile as a ZIP to share: the load order, folders, option values and keys, the profile image,
//! and the files of every enabled mod.
//!
//! ```text
//! obsidian-profile.json   {"obsidianProfile": 1, "name": "...", "profile": {...}}
//! image.png               the profile image, when set
//! mods/<id>/...           one folder per enabled mod, as in the library
//! ```

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::images;
use crate::library::{Library, Profile};
use crate::manifest::load_manifest;
use crate::{ObsidianError, Result};

const FILE: &str = "obsidian-profile.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    obsidian_profile: u32,
    name: String,
    profile: Profile,
}

fn zip_err(e: impl std::fmt::Display) -> ObsidianError {
    ObsidianError::UnsupportedArchive(e.to_string())
}

fn add_dir(z: &mut zip::ZipWriter<std::fs::File>, dir: &Path, prefix: &str) -> Result<()> {
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let name = format!("{prefix}/{}", e.file_name().to_string_lossy());
        if e.file_type()?.is_dir() {
            add_dir(z, &e.path(), &name)?;
        } else {
            z.start_file(name, opts).map_err(zip_err)?;
            z.write_all(&std::fs::read(e.path())?)?;
        }
    }
    Ok(())
}

/// Writes profile `name` to `dest`. Returns how many mods went in with their files.
pub fn export(data: &Path, name: &str, dest: &Path) -> Result<usize> {
    let lib = Library::open(data)?;
    let mut profile = Profile::load_named(data, name)?;
    profile.sync(&lib)?;
    let mut z = zip::ZipWriter::new(std::fs::File::create(dest)?);
    let mut shipped = Vec::new();
    for e in profile.mods.iter().filter(|m| m.enabled) {
        add_dir(&mut z, &lib.mod_dir(&e.id), &format!("mods/{}", e.id))?;
        shipped.push(e.id.clone());
    }
    // Disabled mods stay home.
    profile.mods.retain(|m| shipped.contains(&m.id));
    profile.options.retain(|id, _| profile.mods.iter().any(|m| &m.id == id));
    profile.keys.retain(|id, _| profile.mods.iter().any(|m| &m.id == id));
    // A save id means nothing on another PC.
    profile.save = None;
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    if let Some(img) = images::custom(data, name) {
        let ext = img.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        z.start_file(format!("image.{ext}"), opts).map_err(zip_err)?;
        z.write_all(&std::fs::read(img)?)?;
    }
    z.start_file(FILE, opts).map_err(zip_err)?;
    let header = Header { obsidian_profile: 1, name: name.to_string(), profile };
    z.write_all((serde_json::to_string_pretty(&header)? + "\n").as_bytes())?;
    z.finish().map_err(zip_err)?;
    Ok(shipped.len())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    /// The new profile's name: the one in the ZIP, numbered when taken.
    pub name: String,
    /// Mods installed or updated from the ZIP.
    pub installed: Vec<String>,
    /// Mods already in the library at the same or a newer version, kept as they are.
    pub kept: Vec<String>,
    /// The guardrails the profile came with switched on: `block_achievements`, `freeze_progress`.
    pub guardrails: Vec<String>,
}

/// Adds the profile in `src` as a new profile, installing the mods it carries. It does not become active.
pub fn import(data: &Path, src: &Path) -> Result<Imported> {
    let lib = Library::open(data)?;
    let tmp = tempfile::tempdir()?;
    let mut z = zip::ZipArchive::new(std::fs::File::open(src)?).map_err(zip_err)?;
    z.extract(tmp.path()).map_err(zip_err)?;
    let text = std::fs::read_to_string(tmp.path().join(FILE)).map_err(|_| ObsidianError::NothingToImport(src.display().to_string()))?;
    let header: Header = serde_json::from_str(&text)?;

    let (mut installed, mut kept) = (Vec::new(), Vec::new());
    for e in std::fs::read_dir(tmp.path().join("mods")).into_iter().flatten() {
        let dir = e?.path();
        let m = load_manifest(&dir)?;
        match load_manifest(&lib.mod_dir(&m.id)) {
            Ok(have) if have.version >= m.version => kept.push(m.id),
            _ => {
                lib.install(&dir)?;
                installed.push(m.id);
            }
        }
    }
    installed.sort();
    kept.sort();

    let taken = crate::profiles::list(data)?;
    let free = |n: &str| !taken.iter().any(|t| crate::profiles::same_name(t, n));
    let name = (1..).map(|i| if i == 1 { header.name.clone() } else { format!("{} ({i})", header.name) }).find(|n| free(n)).expect("a free name");
    crate::profiles::create(data, &name, None)?;
    let mut profile = header.profile;
    profile.sync(&lib)?;
    // Blocking other players' mods is for each player to choose, not for a shared file.
    profile.block_public = false;
    // The guardrails only protect the player's own save, so they come along, and the player is told.
    let mut guardrails = Vec::new();
    if profile.block_achievements {
        guardrails.push("block_achievements".to_string());
    }
    if profile.freeze_progress {
        guardrails.push("freeze_progress".to_string());
    }
    profile.save_named(data, &name)?;
    let image = images::TYPES.iter().map(|ext| tmp.path().join(format!("image.{ext}"))).find(|p| p.is_file());
    if let Some(img) = image {
        images::set(data, &name, Some(&img))?;
    }
    Ok(Imported { name, installed, kept, guardrails })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    fn add(lib: &Library, root: &Path, id: &str, version: &str) {
        let d = root.join(format!("stage-{id}-{version}"));
        put(&d, "obsidian.json", format!(r#"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}}}}"#).as_bytes());
        put(&d.join("lua"), "main.lua", b"-- hi");
        lib.install(&d).unwrap();
    }

    #[test]
    fn a_profile_travels_with_its_enabled_mods_options_and_folders() {
        let t = spaced_tempdir();
        let home = t.path().join("home");
        let lib = Library::open(&home).unwrap();
        add(&lib, t.path(), "a.on", "1.0.0");
        add(&lib, t.path(), "b.off", "1.0.0");
        let mut p = Profile::load(&home).unwrap();
        p.sync(&lib).unwrap();
        p.set_enabled("a.on", true).unwrap();
        p.block_public = true;
        p.freeze_progress = true;
        p.options.insert("a.on".into(), [("speed".to_string(), serde_json::json!(3))].into());
        p.create_folder("Main", "#e8b931", &["a.on".to_string()]);
        p.save(&home).unwrap();
        put(t.path(), "pic.png", b"png");
        images::set(&home, "Default", Some(&t.path().join("pic.png"))).unwrap();
        let zip = t.path().join("Default.zip");
        assert_eq!(export(&home, "Default", &zip).unwrap(), 1);

        // A friend with an older "a.on" and a newer copy of nothing else.
        let away = t.path().join("away");
        let other = Library::open(&away).unwrap();
        add(&other, t.path(), "a.on", "0.9.0");
        let r = import(&away, &zip).unwrap();
        assert_eq!(r, Imported { name: "Default (2)".into(), installed: vec!["a.on".into()], kept: vec![], guardrails: vec!["freeze_progress".into()] });
        assert_eq!(load_manifest(&other.mod_dir("a.on")).unwrap().version.to_string(), "1.0.0");
        assert!(!other.mod_dir("b.off").exists(), "disabled mods stay home");
        let q = Profile::load_named(&away, "Default (2)").unwrap();
        assert!(q.mods.iter().any(|m| m.id == "a.on" && m.enabled && m.folder.as_deref() == Some("f1")));
        assert_eq!(q.options["a.on"]["speed"], serde_json::json!(3));
        assert_eq!(q.folders[0].name, "Main");
        assert!(!q.block_public, "blocking other players is each player's own choice");
        assert!(q.freeze_progress, "a guardrail only protects the player, so it comes along");
        assert!(images::custom(&away, "Default (2)").is_some());

        // Importing again keeps the installed copy.
        let r = import(&away, &zip).unwrap();
        assert_eq!((r.name.as_str(), r.kept.as_slice()), ("Default (3)", ["a.on".to_string()].as_slice()));
    }

    #[test]
    fn a_zip_without_a_profile_is_refused() {
        let t = spaced_tempdir();
        let zip = t.path().join("x.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        z.start_file("readme.txt", zip::write::SimpleFileOptions::default()).unwrap();
        z.finish().unwrap();
        assert!(matches!(import(&t.path().join("data"), &zip), Err(ObsidianError::NothingToImport(_))));
    }
}
