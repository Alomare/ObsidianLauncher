//! Finding the HELLDIVERS install and the player's save folder.

use std::path::{Path, PathBuf};

use crate::{ObsidianError, Result};

pub const STEAM_DIR: &str = "steamapps/common/Helldivers";
/// The game's Proton prefix, relative to a Steam library.
const PROTON_PREFIX: &str = "steamapps/compatdata/394510/pfx";
/// The roaming app data folder, relative to a Proton prefix.
const PROTON_APPDATA: &str = "drive_c/users/steamuser/AppData/Roaming";

#[derive(Debug, Clone, PartialEq)]
pub struct GameInstall {
    pub root: PathBuf,
    /// The game's own folder under the roaming app data. With Steam Cloud on it holds only a backup
    /// copy, and it can be empty.
    pub saves_dir: PathBuf,
    /// `userdata/<account>/394510/remote`, one per Steam account: the real save with Steam Cloud on.
    pub cloud_saves: Vec<PathBuf>,
}

impl GameInstall {
    pub fn contents(&self) -> PathBuf {
        self.root.join("contents")
    }
    pub fn exe(&self) -> PathBuf {
        self.root.join("binaries").join("x64").join("helldivers.exe")
    }
}

/// The first library in `libraries` holding the game, else `GameNotFound` naming them all.
pub fn find_in_libraries(libraries: &[PathBuf]) -> Result<PathBuf> {
    libraries
        .iter()
        .map(|lib| lib.join(STEAM_DIR))
        .find(|root| root.join("binaries").join("x64").join("helldivers.exe").is_file())
        .ok_or_else(|| {
            let names: Vec<String> = libraries.iter().map(|l| l.display().to_string()).collect();
            ObsidianError::GameNotFound(if names.is_empty() { "(no Steam library found)".into() } else { names.join("; ") })
        })
}

/// The explicit folder when given, else the game found through Steam.
pub fn find_game(explicit: Option<&Path>) -> Result<GameInstall> {
    let root = match explicit {
        Some(p) => p.to_path_buf(),
        None => find_in_libraries(&steam_libraries())?,
    };
    let appdata = appdata(&root)?;
    Ok(GameInstall { root, saves_dir: appdata.join("Arrowhead").join("Helldivers").join("saves"), cloud_saves: cloud_save_dirs(&steam_roots()) })
}

/// The game's Steam app id.
const APP_ID: &str = "394510";

/// The cloud save folder of every Steam account under `roots`.
pub fn cloud_save_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for root in roots {
        let Ok(accounts) = std::fs::read_dir(root.join("userdata")) else { continue };
        for account in accounts.flatten() {
            let remote = account.path().join(APP_ID).join("remote");
            // The same folder can arrive twice, from the registry and the default path, spelled differently.
            let same = std::fs::canonicalize(&remote).unwrap_or_else(|_| remote.clone());
            if remote.is_dir() && !seen.contains(&same) {
                seen.push(same);
                out.push(remote);
            }
        }
    }
    out
}

/// `OBSIDIAN_APPDATA` (tests), else the roaming app data folder the game sees: the Windows one, or
/// the one in the Proton prefix on Linux.
fn appdata(game_root: &Path) -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("OBSIDIAN_APPDATA") {
        return Ok(p.into());
    }
    if cfg!(windows) {
        return Ok(std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default());
    }
    proton_appdata(game_root, &steam_libraries()).ok_or(ObsidianError::NoProtonPrefix)
}

/// The roaming app data folder in the game's Proton prefix. Steam keeps the prefix in the library
/// that holds the game, so that one goes first; a game folder outside Steam's layout falls back to
/// the other libraries.
pub fn proton_appdata(game_root: &Path, libraries: &[PathBuf]) -> Option<PathBuf> {
    let own = game_root.ancestors().nth(3).map(Path::to_path_buf);
    own.iter().chain(libraries).map(|lib| lib.join(PROTON_PREFIX)).find(|p| p.is_dir()).map(|p| p.join(PROTON_APPDATA))
}

pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam");
        if let Ok(path) = key.and_then(|k| k.get_value::<String, _>("SteamPath")) {
            roots.push(PathBuf::from(path));
        }
        // The machine-wide key, for a Windows account that has not started Steam yet.
        for key in [r"SOFTWARE\WOW6432Node\Valve\Steam", r"SOFTWARE\Valve\Steam"] {
            let key = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(key);
            if let Ok(path) = key.and_then(|k| k.get_value::<String, _>("InstallPath")) {
                roots.push(PathBuf::from(path));
            }
        }
        roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    }
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        roots.push(home.join("snap/steam/common/.local/share/Steam"));
    }
    roots
}

fn steam_libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();
    for root in steam_roots() {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        let found = std::fs::read_to_string(&vdf).map(|t| crate::vdf::library_paths(&t)).unwrap_or_default();
        for lib in std::iter::once(root).chain(found) {
            if !libs.contains(&lib) {
                libs.push(lib);
            }
        }
    }
    libs
}

/// The game's process name.
pub const EXE: &str = "helldivers.exe";

/// Whether HELLDIVERS is running.
/// Deploy and purge rewrite game files, so they wait until the game is closed.
pub fn ensure_closed() -> Result<()> {
    if is_running() { Err(crate::ObsidianError::GameRunning) } else { Ok(()) }
}

pub fn is_running() -> bool {
    is_running_named(EXE)
}

/// Whether a process with this file name runs, ignoring case.
pub fn is_running_named(name: &str) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    // One process list for the whole run: the app asks every 2 seconds, and a refresh of a kept
    // list costs less than a new one each time.
    static LIST: std::sync::Mutex<Option<System>> = std::sync::Mutex::new(None);
    let mut list = LIST.lock().unwrap_or_else(|p| p.into_inner());
    if list.is_none() {
        // A system that can't list processes counts as "not running", so deploys stay possible.
        let Ok(s) = System::new() else { return false };
        *list = Some(s);
    }
    let Some(s) = list.as_mut() else { return false };
    #[cfg(windows)]
    {
        s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        s.processes().values().any(|p| p.name().to_string_lossy().eq_ignore_ascii_case(name))
    }
    // Under Proton the game is a Wine process: Linux cuts process names at 15 characters, and Wine
    // puts the Windows path of the exe first on the command line, so that counts too.
    #[cfg(not(windows))]
    {
        s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::OnlyIfNotSet));
        s.processes().values().any(|p| names_program(&p.name().to_string_lossy(), p.cmd().first().map(|a| a.to_string_lossy()).as_deref(), name))
    }
}

/// Whether a process named `comm`, started with `arg0` first on the command line, is `name`: by
/// name, or by the file name of `arg0` as Linux or Windows spells paths.
#[cfg_attr(windows, allow(dead_code))]
fn names_program(comm: &str, arg0: Option<&str>, name: &str) -> bool {
    comm.eq_ignore_ascii_case(name) || arg0.and_then(|a| a.rsplit(['/', '\\']).next()).is_some_and(|f| f.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    #[test]
    fn finds_game_in_the_library_that_has_it() {
        let a = spaced_tempdir();
        let b = spaced_tempdir();
        put(b.path(), "steamapps/common/Helldivers/binaries/x64/helldivers.exe", b"exe");
        let libs = vec![a.path().to_path_buf(), b.path().to_path_buf()];
        assert_eq!(find_in_libraries(&libs).unwrap(), b.path().join(STEAM_DIR));
        assert!(find_in_libraries(&libs[..1]).is_err());
    }

    #[test]
    fn cloud_saves_are_found_per_account() {
        let steam = spaced_tempdir();
        put(steam.path(), "userdata/111/394510/remote/helldivers_save", b"save");
        put(steam.path(), "userdata/222/394510/remote/helldivers_save", b"save");
        put(steam.path(), "userdata/333/570/remote/other_game", b"save");
        let mut found = cloud_save_dirs(&[steam.path().to_path_buf()]);
        found.sort();
        let want = |id: &str| steam.path().join("userdata").join(id).join("394510").join("remote");
        assert_eq!(found, [want("111"), want("222")]);
        assert!(cloud_save_dirs(&[steam.path().join("missing")]).is_empty());
        let twice = [steam.path().to_path_buf(), steam.path().join(".")];
        assert_eq!(cloud_save_dirs(&twice).len(), 2, "one folder per account, however the root is spelled");
    }

    #[test]
    fn explicit_folder_is_used_as_is() {
        let lib = spaced_tempdir();
        put(lib.path(), "steamapps/compatdata/394510/pfx/system.reg", b"");
        let root = lib.path().join(STEAM_DIR);
        assert_eq!(find_game(Some(&root)).unwrap().root, root);
    }

    #[test]
    fn the_proton_prefix_is_the_one_beside_the_game() {
        let (a, b) = (spaced_tempdir(), spaced_tempdir());
        put(a.path(), "steamapps/compatdata/394510/pfx/system.reg", b"");
        put(b.path(), "steamapps/compatdata/394510/pfx/system.reg", b"");
        let libs = vec![a.path().to_path_buf(), b.path().to_path_buf()];
        let found = proton_appdata(&b.path().join(STEAM_DIR), &libs).unwrap();
        assert_eq!(found, b.path().join(PROTON_PREFIX).join(PROTON_APPDATA));
    }

    #[test]
    fn a_game_outside_steam_uses_a_prefix_from_any_library() {
        let (lib, elsewhere) = (spaced_tempdir(), spaced_tempdir());
        put(lib.path(), "steamapps/compatdata/394510/pfx/system.reg", b"");
        let libs = vec![lib.path().to_path_buf()];
        let found = proton_appdata(&elsewhere.path().join("Helldivers"), &libs).unwrap();
        assert!(found.starts_with(lib.path()) && found.ends_with("AppData/Roaming"));
        assert_eq!(proton_appdata(&elsewhere.path().join("Helldivers"), &[]), None);
    }
    #[test]
    fn running_process_is_found_by_name() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_uppercase();
        // Linux keeps 15 characters of a process name. The game's name has to fit, or the check
        // would never find it under Proton.
        assert!(EXE.len() <= 15);
        let name: String = name.chars().take(if cfg!(windows) { usize::MAX } else { 15 }).collect();
        assert!(is_running_named(&name));
        assert!(!is_running_named("no-such-process-obsidian.exe"));
    }

    #[test]
    fn a_wine_process_is_known_by_the_windows_path_on_the_command_line() {
        assert!(names_program("helldivers.exe", None, EXE));
        assert!(names_program("wine64-preloade", Some(r"Z:\home\deck\.local\share\Steam\steamapps\common\Helldivers\binaries\x64\helldivers.exe"), EXE));
        assert!(names_program("x", Some("/games/Helldivers/binaries/x64/HELLDIVERS.EXE"), EXE));
        assert!(!names_program("helldivers", Some("/usr/bin/helldivers"), EXE));
        assert!(!names_program("x", Some(""), EXE));
    }
}
