//! Square images for mods and profiles. A mod can name one in the manifest (`"image": "image.png"`,
//! inside the mod folder). A profile image is set by the player and kept in
//! `<data>/images/profiles/<name>.<ext>`.

use std::path::{Path, PathBuf};

use crate::library::Library;
use crate::manifest::Manifest;
use crate::{ObsidianError, Result};

pub const TYPES: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
pub const MAX_BYTES: u64 = 4 * 1024 * 1024;

fn dir(data: &Path) -> PathBuf {
    data.join("images").join("profiles")
}

fn allowed(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| TYPES.contains(&e.to_ascii_lowercase().as_str()))
}

/// The image of a profile, if the player set one.
pub fn custom(data: &Path, key: &str) -> Option<PathBuf> {
    TYPES.iter().map(|ext| dir(data).join(format!("{key}.{ext}"))).find(|p| p.is_file())
}

/// The image a mod's manifest names, if that file is an image inside the mod folder.
pub fn from_manifest(lib: &Library, m: &Manifest) -> Option<PathBuf> {
    let name = m.extra.get("image")?.as_str()?;
    let root = std::fs::canonicalize(lib.mod_dir(&m.id)).ok()?;
    let p = std::fs::canonicalize(root.join(name)).ok()?;
    (p.starts_with(&root) && p.is_file() && allowed(&p)).then_some(p)
}

/// Sets the image of a profile from `from` (a copy is kept), or removes it with `None`.
pub fn set(data: &Path, key: &str, from: Option<&Path>) -> Result<()> {
    if let Some(src) = from {
        let size = std::fs::metadata(src)?.len();
        if !allowed(src) || size > MAX_BYTES {
            return Err(ObsidianError::BadImage(src.display().to_string()));
        }
    }
    remove(data, key)?;
    if let Some(src) = from {
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
        std::fs::create_dir_all(dir(data))?;
        std::fs::copy(src, dir(data).join(format!("{key}.{ext}")))?;
    }
    Ok(())
}

pub fn remove(data: &Path, key: &str) -> Result<()> {
    while let Some(p) = custom(data, key) {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

/// Keeps the image when a profile is renamed.
pub fn rename(data: &Path, from: &str, to: &str) -> Result<()> {
    if let Some(p) = custom(data, from) {
        let ext = p.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        std::fs::rename(&p, dir(data).join(format!("{to}.{ext}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    #[test]
    fn player_images_are_copied_replaced_renamed_and_removed() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        put(t.path(), "a.PNG", b"png");
        put(t.path(), "b.webp", b"webp");
        set(&data, "Main", Some(&t.path().join("a.PNG"))).unwrap();
        assert_eq!(custom(&data, "Main"), Some(data.join("images/profiles/Main.png")));
        set(&data, "Main", Some(&t.path().join("b.webp"))).unwrap();
        assert_eq!(custom(&data, "Main"), Some(data.join("images/profiles/Main.webp")), "one image per profile");
        rename(&data, "Main", "Other").unwrap();
        assert_eq!(custom(&data, "Main"), None);
        assert!(custom(&data, "Other").is_some());
        set(&data, "Other", None).unwrap();
        assert_eq!(custom(&data, "Other"), None);
    }

    #[test]
    fn other_files_are_refused() {
        let t = spaced_tempdir();
        put(t.path(), "a.exe", b"x");
        assert!(matches!(set(t.path(), "m", Some(&t.path().join("a.exe"))), Err(ObsidianError::BadImage(_))));
        let big = t.path().join("big.png");
        std::fs::File::create(&big).unwrap().set_len(MAX_BYTES + 1).unwrap();
        assert!(matches!(set(t.path(), "m", Some(&big)), Err(ObsidianError::BadImage(_))));
    }

    #[test]
    fn a_mod_shows_the_image_inside_the_mod_folder() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        let json = |image: &str| format!(r#"{{"obsidian":1,"id":"a.b","version":"1.0.0","name":{{"en":"a"}},"image":"{image}"}}"#);
        let mut m: Manifest = serde_json::from_str(&json("art/icon.png")).unwrap();
        assert_eq!(from_manifest(&lib, &m), None, "the file is missing");
        put(&lib.mod_dir("a.b").join("art"), "icon.png", b"png");
        assert!(from_manifest(&lib, &m).unwrap().ends_with("icon.png"));
        put(&data, "secret.png", b"png");
        m = serde_json::from_str(&json("../../secret.png")).unwrap();
        assert_eq!(from_manifest(&lib, &m), None, "outside the mod folder");
    }
}
