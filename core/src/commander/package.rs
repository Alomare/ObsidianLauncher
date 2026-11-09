//! The Commander package: a ZIP with `commander.json`, the server and the interface, installed into
//! `<data>/commander/package`. A new install replaces the old one whole, and only after it unpacked.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use super::{Installed, PROTOCOL, State, dir};
use crate::{ObsidianError, Result};

const MANIFEST: &str = "commander.json";
/// A package is a server and a web interface; anything near this size is something else.
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    protocol: u32,
    server: String,
    ui: String,
}

pub fn package_dir(data: &Path) -> PathBuf {
    dir(data).join("package")
}

fn bad(why: impl Into<String>) -> ObsidianError {
    ObsidianError::CommanderPackage(why.into())
}

/// A plain relative name inside the package, never `..` or a root.
fn safe_name(name: &str) -> bool {
    let p = Path::new(name);
    !name.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_)))
}

fn read_manifest(folder: &Path) -> Result<Manifest> {
    let text = std::fs::read_to_string(folder.join(MANIFEST)).map_err(|_| bad("no commander.json"))?;
    let m: Manifest = serde_json::from_str(&text).map_err(|e| bad(format!("commander.json: {e}")))?;
    if m.protocol != PROTOCOL {
        return Err(bad(format!("protocol {} (this Obsidian speaks {PROTOCOL})", m.protocol)));
    }
    if !safe_name(&m.server) || !folder.join(&m.server).is_file() {
        return Err(bad(format!("no server file {:?}", m.server)));
    }
    if !safe_name(&m.ui) || !folder.join(&m.ui).join("index.html").is_file() {
        return Err(bad(format!("no interface in {:?}", m.ui)));
    }
    Ok(m)
}

/// Unpacks the ZIP into `to`, refusing names that leave the folder.
fn unpack(zip_path: &Path, to: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| bad(e.to_string()))?;
    let mut total = 0u64;
    for i in 0..z.len() {
        let mut entry = z.by_index(i).map_err(|e| bad(e.to_string()))?;
        let name = entry.name().map_err(|e| bad(e.to_string()))?.replace('\\', "/");
        if !safe_name(name.trim_end_matches('/')) {
            return Err(bad(format!("unsafe path {name:?}")));
        }
        let out = to.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        total += entry.size();
        if total > MAX_UNPACKED {
            return Err(bad("larger than 256 MB unpacked"));
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        std::fs::write(&out, bytes)?;
    }
    Ok(())
}

/// The package's root inside an unpacked folder: the folder itself, or its only subfolder when a
/// ZIP wraps everything in one.
fn root_of(folder: &Path) -> PathBuf {
    if folder.join(MANIFEST).is_file() {
        return folder.to_path_buf();
    }
    let subs: Vec<PathBuf> = std::fs::read_dir(folder).into_iter().flatten().flatten().map(|e| e.path()).collect();
    match subs.as_slice() {
        [only] if only.is_dir() && only.join(MANIFEST).is_file() => only.clone(),
        _ => folder.to_path_buf(),
    }
}

/// Installs a package ZIP, or a package folder, replacing the installed one. Refused while the
/// server runs from it (the caller stops it first).
pub fn install(data: &Path, source: &Path) -> Result<Installed> {
    let base = dir(data);
    std::fs::create_dir_all(&base)?;
    let staging = base.join("package.new");
    let _ = std::fs::remove_dir_all(&staging);
    let done = (|| -> Result<Installed> {
        if source.is_dir() {
            crate::paths::copy_tree(source, &staging, false)?;
        } else {
            unpack(source, &staging)?;
        }
        let root = root_of(&staging);
        let m = read_manifest(&root)?;
        let target = package_dir(data);
        let old = base.join("package.old");
        let _ = std::fs::remove_dir_all(&old);
        if target.exists() {
            std::fs::rename(&target, &old)?;
        }
        if let Err(e) = std::fs::rename(&root, &target) {
            // Put the old one back, so a failed install leaves a working package.
            if old.exists() {
                let _ = std::fs::rename(&old, &target);
            }
            return Err(e.into());
        }
        let _ = std::fs::remove_dir_all(&old);
        Ok(Installed { version: m.version, protocol: m.protocol, server: m.server, ui: m.ui })
    })();
    let _ = std::fs::remove_dir_all(&staging);
    let installed = done?;
    let mut s = State::load(data);
    s.package = Some(installed.clone());
    s.save(data)?;
    Ok(installed)
}

/// Removes the package. The wars this PC hosted stay in `<data>/commander/server`.
pub fn uninstall(data: &Path) -> Result<()> {
    match std::fs::remove_dir_all(package_dir(data)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let mut s = State::load(data);
    s.package = None;
    s.save(data)
}

/// The installed package, checked against the files: None when the folder was removed or broken.
pub fn installed(data: &Path) -> Option<Installed> {
    let s = State::load(data);
    let i = s.package?;
    let d = package_dir(data);
    (d.join(&i.server).is_file() && d.join(&i.ui).join("index.html").is_file()).then_some(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(path: &Path, files: &[(&str, &[u8])]) {
        let f = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        for (name, bytes) in files {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
    }

    const GOOD: &[u8] = br#"{"name":"Obsidian Commander","version":"0.1.0","protocol":1,"server":"commander-server.exe","ui":"ui"}"#;

    #[test]
    fn installs_a_good_package_and_replaces_it() {
        let t = tempfile::tempdir().unwrap();
        let z = t.path().join("c.zip");
        zip_of(&z, &[("commander.json", GOOD), ("commander-server.exe", b"MZ"), ("ui/index.html", b"<html>")]);
        let i = install(&t.path().join("data"), &z).unwrap();
        assert_eq!(i.version, "0.1.0");
        assert_eq!(installed(&t.path().join("data")), Some(i));
        let z2 = t.path().join("c2.zip");
        let newer = String::from_utf8_lossy(GOOD).replace("0.1.0", "0.2.0");
        zip_of(&z2, &[("Commander/commander.json", newer.as_bytes()), ("Commander/commander-server.exe", b"MZ"), ("Commander/ui/index.html", b"<html>")]);
        assert_eq!(install(&t.path().join("data"), &z2).unwrap().version, "0.2.0", "a ZIP wrapped in one folder");
        uninstall(&t.path().join("data")).unwrap();
        assert_eq!(installed(&t.path().join("data")), None);
    }

    #[test]
    fn refuses_bad_packages_and_keeps_the_old_one() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let good = t.path().join("good.zip");
        zip_of(&good, &[("commander.json", GOOD), ("commander-server.exe", b"MZ"), ("ui/index.html", b"<html>")]);
        install(&data, &good).unwrap();
        let cases: [(&str, Vec<(&str, &[u8])>); 4] = [
            ("no manifest", vec![("x.txt", b"x")]),
            ("wrong protocol", vec![("commander.json", br#"{"version":"9","protocol":99,"server":"s.exe","ui":"ui"}"#), ("s.exe", b"MZ"), ("ui/index.html", b"x")]),
            ("no server", vec![("commander.json", GOOD), ("ui/index.html", b"x")]),
            ("escaping path", vec![("commander.json", GOOD), ("../evil.txt", b"x")]),
        ];
        for (what, files) in cases {
            let z = t.path().join("bad.zip");
            zip_of(&z, &files);
            assert!(matches!(install(&data, &z), Err(ObsidianError::CommanderPackage(_))), "{what}");
            assert_eq!(installed(&data).map(|i| i.version), Some("0.1.0".to_string()), "{what}: the old package stays");
        }
        assert!(!t.path().join("evil.txt").exists());
    }
}
