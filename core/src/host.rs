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

/// Starts `program` with `args` once this process has ended, for the restart after an update.
/// The new copy gets the system environment, not the paths into this AppImage's mount (gone with
/// this process), and starts after this one has let go of the single-instance lock.
#[cfg(unix)]
pub fn relaunch_after_exit(program: &Path, args: &[OsString]) -> std::io::Result<()> {
    // The folder the AppImage was started from: an AppImage runs inside its mount.
    let dir = std::env::var_os("OWD").filter(|d| Path::new(d).is_dir());
    relaunch_after(std::process::id(), program, args, dir.as_deref().map(Path::new))
}

/// Starts `program` once process `pid` has ended, or after 10 s whatever happens.
#[cfg(unix)]
fn relaunch_after(pid: u32, program: &Path, args: &[OsString], dir: Option<&Path>) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    const WAIT: &str = r#"i=0; while kill -0 "$1" 2>/dev/null && [ $i -lt 50 ]; do sleep 0.2; i=$((i+1)); done; shift; exec "$@""#;
    let mut cmd = command("/bin/sh");
    cmd.arg("-c").arg(WAIT).arg("obsidian-restart").arg(pid.to_string()).arg(program).args(args);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    cmd.spawn().map(|_| ())
}

// AppImage paths are split on ':', as on Linux; Windows splits path lists on ';'.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn the_relaunch_waits_for_the_old_process() {
        let dir = std::env::temp_dir().join(format!("obsidian-relaunch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut old = Command::new("sleep").arg("1").spawn().unwrap();
        let args = [OsString::from("-c"), OsString::from("pwd > started")];
        relaunch_after(old.id(), Path::new("/bin/sh"), &args, Some(&dir)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));
        assert!(!dir.join("started").exists(), "started while the old process ran");
        old.wait().unwrap();
        let started = (0..50).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            dir.join("started").exists()
        });
        assert!(started, "never started");
        let _ = std::fs::remove_dir_all(&dir);
    }

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
