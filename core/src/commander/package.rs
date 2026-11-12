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

/// Where the package comes from: the package releases, each with an `obsidian-commander-<version>.zip`
/// for Windows and a `commander-linux-x86_64-<version>.zip` for Linux. The Linux name shares no
/// prefix with the Windows one, so Obsidian releases before Linux support never pick it.
pub const SOURCE: &str = "https://github.com/Alomare/ObsidianCommander";
#[cfg(windows)]
const ASSET_PREFIX: &str = "obsidian-commander-";
#[cfg(not(windows))]
const ASSET_PREFIX: &str = "commander-linux-x86_64-";

/// How a program for this system starts: a Windows exe, or an ELF file on Linux.
#[cfg(windows)]
const PROGRAM_MAGIC: &[u8] = b"MZ";
#[cfg(not(windows))]
const PROGRAM_MAGIC: &[u8] = b"\x7fELF";

/// Whether `path` is a program this system can start, going by the first bytes.
fn native_program(path: &Path) -> bool {
    let mut head = [0u8; 4];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    head[..n].starts_with(PROGRAM_MAGIC)
}

/// The public key downloaded packages are signed for: the key of Obsidian updates
/// (`plugins.updater.pubkey` in `tauri.conf.json`). The package holds a program that Obsidian
/// starts, so a download is installed only when its signature was made with that key.
const PACKAGE_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IENFMDk3NEVCQ0Q4M0YwMTAKUldRUThJUE42M1FKem84YlozRG44OUJuenJvRzNGeE9sajE4R2NpeHFXY3NoZEtUdEtuSmtIbFkK";

/// Downloads the newest released package from GitHub, checks its signature and installs it.
pub fn fetch(data: &Path, github: &crate::updates::GitHub) -> Result<Installed> {
    let repo = crate::updates::github_repo(SOURCE).expect("a GitHub address");
    let dir = tempfile::tempdir()?;
    let zip = dir.path().join("commander.zip");
    let (_, sig) = github.download_latest_zip(&repo, ASSET_PREFIX, &zip)?;
    let sig = sig.ok_or(ObsidianError::CommanderUnsigned)?;
    verify_signed(&std::fs::read(&zip)?, &sig, PACKAGE_KEY)?;
    install(data, &zip)
}

/// Standard base64 to bytes; None for anything else.
fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.trim().trim_end_matches('=').bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Whether `signature` (the text of a `.sig` file from `tauri signer sign`) is a signature of
/// `bytes` made with the private half of `key` (a public key as `tauri.conf.json` holds it).
fn verify_signed(bytes: &[u8], signature: &str, key: &str) -> Result<()> {
    let unsigned = || ObsidianError::CommanderUnsigned;
    let text = |b64: &str| unbase64(b64).and_then(|b| String::from_utf8(b).ok());
    let key = text(key).and_then(|t| minisign_verify::PublicKey::decode(&t).ok()).ok_or_else(unsigned)?;
    let signature = text(signature).and_then(|t| minisign_verify::Signature::decode(&t).ok()).ok_or_else(unsigned)?;
    key.verify(bytes, &signature, false).map_err(|_| unsigned())
}

/// The version of a newer released package than the one installed, if there is one.
pub fn newer(data: &Path, github: &crate::updates::GitHub) -> Result<Option<String>> {
    let Some(installed) = installed(data) else { return Ok(None) };
    let repo = crate::updates::github_repo(SOURCE).expect("a GitHub address");
    let latest = github.newest_version(&repo)?;
    Ok(is_newer(&installed.version, &latest.to_string()).then(|| latest.to_string()))
}

/// Whether `latest` is past `installed`. An installed version that is not a version (a local build)
/// counts as older.
fn is_newer(installed: &str, latest: &str) -> bool {
    let v = |s: &str| semver::Version::parse(s.trim().trim_start_matches(['v', 'V'])).ok();
    match (v(installed), v(latest)) {
        (Some(i), Some(l)) => l > i,
        (None, Some(_)) => true,
        _ => false,
    }
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
    if !native_program(&folder.join(&m.server)) {
        return Err(bad(format!("the server file {:?} is made for another system", m.server)));
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
        if total + entry.size() > MAX_UNPACKED {
            return Err(bad("larger than 256 MB unpacked"));
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        // Counted as read, not as the entry header says: a header can understate the size.
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        (&mut entry).take(MAX_UNPACKED - total + 1).read_to_end(&mut bytes)?;
        total += bytes.len() as u64;
        if total > MAX_UNPACKED {
            return Err(bad("larger than 256 MB unpacked"));
        }
        std::fs::write(&out, bytes)?;
    }
    Ok(())
}

/// The package root inside an unpacked folder: the folder itself, or the only subfolder when a
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
        // A ZIP keeps no run permission that Obsidian reads back, so the server gets one here.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(target.join(&m.server), std::fs::Permissions::from_mode(0o755))?;
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

    #[test]
    fn a_newer_release_is_one_past_the_installed_version() {
        assert!(is_newer("0.3.1", "0.3.2"));
        assert!(is_newer("v0.3.1", "0.10.0"));
        assert!(!is_newer("0.3.2", "0.3.2"));
        assert!(!is_newer("0.4.0", "0.3.2"));
        assert!(is_newer("local build", "0.3.2"));
        assert!(!is_newer("0.3.2", "nightly"));
    }

    /// A key made for this test with `tauri signer generate`, and its signature of `SIGNED`.
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDYwM0VDODRENUQ5RTYyQjAKUldTd1lwNWRUY2crWUFBNjAzOSszMjltdXBRUzZINDY0S0tDT21oendaUU8zYkxMV295dXZ5aDAK";
    const TEST_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTd1lwNWRUY2crWURmandvSW1aUGk2S1pBMFE5NkZSdWd5aGE4R0wyQXNGdmUyYk5kQVRCdmt3d2dIcjFEeWVUYnFOb0NLZUpxSnowL0sxZGZQcU01ZmtjUnFiYm0wcUFJPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0NDE5OTMxCWZpbGU6cGtnLnppcAorQW94TzlrRE5hRUhQRnltS0lqNnpDVEY2WDd4dFBydUlXY1J2N0p4bkUrQVpXVFd0MFRpUHBXV21OdjMzL3lDVkRCU0dva2t5UXROc3ZaMGFRZFlCUT09Cg==";
    const SIGNED: &[u8] = b"a test package";

    #[test]
    fn a_download_needs_a_signature_from_the_right_key_over_the_same_bytes() {
        verify_signed(SIGNED, TEST_SIG, TEST_KEY).unwrap();
        assert!(matches!(verify_signed(b"a test package.", TEST_SIG, TEST_KEY), Err(ObsidianError::CommanderUnsigned)), "changed bytes");
        assert!(matches!(verify_signed(SIGNED, TEST_SIG, PACKAGE_KEY), Err(ObsidianError::CommanderUnsigned)), "another key");
        assert!(matches!(verify_signed(SIGNED, "not a signature", TEST_KEY), Err(ObsidianError::CommanderUnsigned)));
        assert!(matches!(verify_signed(SIGNED, "", TEST_KEY), Err(ObsidianError::CommanderUnsigned)));
        assert_eq!(unbase64("TWFueQ==").as_deref(), Some(&b"Many"[..]));
        assert_eq!(unbase64("TQ=="), Some(b"M".to_vec()));
        assert_eq!(unbase64("M*"), None);
        // The key in this file reads as a key, and is the one the app updates are checked with.
        let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../app/src-tauri/tauri.conf.json")).unwrap();
        assert!(conf.contains(PACKAGE_KEY), "PACKAGE_KEY is the updater key");
        let text = String::from_utf8(unbase64(PACKAGE_KEY).unwrap()).unwrap();
        assert!(minisign_verify::PublicKey::decode(&text).is_ok());
    }

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
        zip_of(&z, &[("commander.json", GOOD), ("commander-server.exe", PROGRAM_MAGIC), ("ui/index.html", b"<html>")]);
        let i = install(&t.path().join("data"), &z).unwrap();
        assert_eq!(i.version, "0.1.0");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(package_dir(&t.path().join("data")).join(&i.server)).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "the server can run");
        }
        assert_eq!(installed(&t.path().join("data")), Some(i));
        let z2 = t.path().join("c2.zip");
        let newer = String::from_utf8_lossy(GOOD).replace("0.1.0", "0.2.0");
        zip_of(&z2, &[("Commander/commander.json", newer.as_bytes()), ("Commander/commander-server.exe", PROGRAM_MAGIC), ("Commander/ui/index.html", b"<html>")]);
        assert_eq!(install(&t.path().join("data"), &z2).unwrap().version, "0.2.0", "a ZIP wrapped in one folder");
        uninstall(&t.path().join("data")).unwrap();
        assert_eq!(installed(&t.path().join("data")), None);
    }

    #[test]
    fn refuses_bad_packages_and_keeps_the_old_one() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        let good = t.path().join("good.zip");
        zip_of(&good, &[("commander.json", GOOD), ("commander-server.exe", PROGRAM_MAGIC), ("ui/index.html", b"<html>")]);
        install(&data, &good).unwrap();
        #[cfg(windows)]
        const OTHER_SYSTEM: &[u8] = b"ELF";
        #[cfg(not(windows))]
        const OTHER_SYSTEM: &[u8] = b"MZ";
        let cases: [(&str, Vec<(&str, &[u8])>); 5] = [
            ("no manifest", vec![("x.txt", b"x")]),
            ("server for another system", vec![("commander.json", GOOD), ("commander-server.exe", OTHER_SYSTEM), ("ui/index.html", b"x")]),
            ("wrong protocol", vec![("commander.json", br#"{"version":"9","protocol":99,"server":"s.exe","ui":"ui"}"#), ("s.exe", PROGRAM_MAGIC), ("ui/index.html", b"x")]),
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
