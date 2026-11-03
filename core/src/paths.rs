//! Where Obsidian keeps the library, profile, backups and state.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RESOURCES: OnceLock<PathBuf> = OnceLock::new();

/// Tells core where the app's shipped files are, on Linux: packages keep them under `usr/lib`, which
/// only the app can work out. Windows keeps them beside the exe.
pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCES.set(dir);
}

pub fn resource_dir() -> Option<&'static Path> {
    RESOURCES.get().map(PathBuf::as_path)
}

const RENAME_TRIES: u32 = 8;

/// `std::fs::rename`, tried again for a moment when Windows refuses it: an antivirus or the search
/// indexer often holds a new file open just after it's written.
pub fn rename_retrying(from: &Path, to: &Path) -> std::io::Result<()> {
    for _ in 1..RENAME_TRIES {
        match std::fs::rename(from, to) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => std::thread::sleep(std::time::Duration::from_millis(125)),
            other => return other,
        }
    }
    std::fs::rename(from, to)
}

/// Writes through a temporary file beside `path`, so a full disk or a power cut leaves the old file
/// whole instead of half of the new one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_through(path, &with_suffix(path, ".obsidian-tmp"), bytes)
}

/// `path` with `suffix` added to the file name, as in `settings.txt.tmp`.
pub fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// `write_atomic` through a temporary file of the caller's choosing, for files another program
/// also writes that way.
pub fn write_through(path: &Path, tmp: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let written = std::fs::File::create(tmp).and_then(|mut f| f.write_all(bytes).and_then(|_| f.sync_all())).and_then(|_| rename_retrying(tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    written
}

/// The explicit folder, else `data` beside a portable copy (marked by `portable.txt`),
/// else the per-user app data folder.
pub fn data_dir(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf))
        && dir.join("portable.txt").is_file()
    {
        return dir.join("data");
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    base.unwrap_or_default().join(crate::settings::DIR)
}
