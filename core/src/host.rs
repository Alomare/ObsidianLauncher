//! Programs of the system that the app starts: xdg-open, pkexec, firewall-cmd, the Commander server.
//!
//! An AppImage runs with its own folder first in `LD_LIBRARY_PATH`, `PATH`, `PYTHONHOME` and more.
//! A system program that inherits them loads the bundled libraries in place of its own and fails
//! (KDE's `kde-open` stops on the bundled OpenSSL; `firewall-cmd` on `PYTHONHOME`). The commands made
//! here get the environment without the AppImage's entries.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

/// A command for a program of the system, without the AppImage's environment.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    clean(&mut cmd);
    cmd
}

/// Takes the AppImage's entries out of `cmd`'s environment. Nothing changes outside an AppImage.
pub fn clean(cmd: &mut Command) {
    let Some(appdir) = std::env::var_os("APPDIR").filter(|d| !d.is_empty()) else { return };
    for (key, value) in cleaned(Path::new(&appdir), std::env::vars_os()) {
        match value {
            Some(v) => cmd.env(key, v),
            None => cmd.env_remove(key),
        };
    }
}

/// The variables to change: a new value, or `None` to remove. Path lists lose the entries inside
/// `appdir`; a variable left empty, or a single value inside `appdir`, goes.
fn cleaned(appdir: &Path, vars: impl Iterator<Item = (OsString, OsString)>) -> Vec<(OsString, Option<OsString>)> {
    // Set by the AppImage runtime itself.
    const OWN: [&str; 4] = ["APPDIR", "APPIMAGE", "ARGV0", "OWD"];
    let inside = |p: &Path| p.starts_with(appdir);
    let mut out = Vec::new();
    for (key, value) in vars {
        if key.to_str().is_some_and(|k| OWN.contains(&k)) {
            out.push((key, None));
            continue;
        }
        if !value.to_string_lossy().contains(&*appdir.to_string_lossy()) {
            continue;
        }
        let parts: Vec<_> = std::env::split_paths(&value).collect();
        let kept: Vec<_> = parts.iter().filter(|p| !p.as_os_str().is_empty() && !inside(p)).collect();
        if kept.is_empty() {
            out.push((key, None));
        } else if kept.len() < parts.len() {
            out.push((key, std::env::join_paths(kept).ok()));
        }
    }
    out
}

// AppImage paths are split on ':', as on Linux; Windows splits path lists on ';'.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn run(vars: &[(&str, &str)]) -> Vec<(String, Option<String>)> {
        let vars = vars.iter().map(|(k, v)| (OsString::from(k), OsString::from(v)));
        let mut out: Vec<_> = cleaned(Path::new("/tmp/.mount_Obs123"), vars)
            .into_iter()
            .map(|(k, v)| (k.into_string().unwrap(), v.map(|v| v.into_string().unwrap())))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn appimage_entries_go() {
        let out = run(&[
            ("LD_LIBRARY_PATH", "/tmp/.mount_Obs123/usr/lib/:/tmp/.mount_Obs123/lib/:"),
            ("PATH", "/tmp/.mount_Obs123/usr/bin/:/home/u/.local/bin:/usr/bin"),
            ("PYTHONHOME", "/tmp/.mount_Obs123/usr/"),
            ("GDK_PIXBUF_MODULE_FILE", "/tmp/.mount_Obs123//usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"),
            ("XDG_DATA_DIRS", "/tmp/.mount_Obs123/usr/share/:/usr/share:/var/lib/flatpak/exports/share"),
            ("APPIMAGE", "/home/u/Applications/Obsidian.AppImage"),
            ("HOME", "/home/u"),
            ("KDE_SESSION_VERSION", "6"),
        ]);
        assert_eq!(
            out,
            vec![
                ("APPIMAGE".into(), None),
                ("GDK_PIXBUF_MODULE_FILE".into(), None),
                ("LD_LIBRARY_PATH".into(), None),
                ("PATH".into(), Some("/home/u/.local/bin:/usr/bin".into())),
                ("PYTHONHOME".into(), None),
                ("XDG_DATA_DIRS".into(), Some("/usr/share:/var/lib/flatpak/exports/share".into())),
            ]
        );
    }

    #[test]
    fn a_similar_folder_stays() {
        assert!(run(&[("PATH", "/tmp/.mount_Obs1234/bin:/usr/bin")]).is_empty());
    }
}
