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
    let base = base.unwrap_or_default();
    adopt_legacy_data_dir(&base.join(crate::settings::LEGACY_DIR), &base.join(crate::settings::DIR))
}

/// The data folder to use when the product was renamed: the library of an older copy moves to the new
/// name on the first start, so nothing is left behind and nothing is imported twice. A folder that
/// can't be moved right now (something holds a file open) stays where it is and is used as it is;
/// the move is tried again at the next start.
fn adopt_legacy_data_dir(old: &Path, new: &Path) -> PathBuf {
    if new.exists() || !old.is_dir() {
        return new.to_path_buf();
    }
    match rename_retrying(old, new) {
        Ok(()) => new.to_path_buf(),
        Err(_) => old.to_path_buf(),
    }
}

/// Copies the files of `from` into `to`, making folders as needed. With `newer_only` a file that `to`
/// has already is replaced only when the one in `from` was written later.
pub fn copy_tree(from: &Path, to: &Path, newer_only: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type()?.is_dir() {
            copy_tree(&src, &dst, newer_only)?;
            continue;
        }
        let wanted = !newer_only
            || match (src.metadata().and_then(|m| m.modified()), dst.metadata().and_then(|m| m.modified())) {
                (Ok(a), Ok(b)) => a > b,
                (_, Err(_)) => true,
                (Err(_), Ok(_)) => false,
            };
        if wanted {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    #[test]
    fn an_older_data_folder_moves_to_the_new_name() {
        let t = spaced_tempdir();
        let (old, new) = (t.path().join(crate::settings::LEGACY_DIR), t.path().join(crate::settings::DIR));
        put(&old, "library/mod/obsidian.json", b"{}");
        put(&old, "profile.json", b"p");
        assert_eq!(adopt_legacy_data_dir(&old, &new), new);
        assert!(!old.exists(), "moved, not copied");
        assert_eq!(std::fs::read(new.join("profile.json")).unwrap(), b"p");
        assert!(new.join("library/mod/obsidian.json").is_file());
    }

    #[test]
    fn a_new_data_folder_is_never_overwritten() {
        let t = spaced_tempdir();
        let (old, new) = (t.path().join(crate::settings::LEGACY_DIR), t.path().join(crate::settings::DIR));
        put(&old, "profile.json", b"old");
        put(&new, "profile.json", b"new");
        assert_eq!(adopt_legacy_data_dir(&old, &new), new);
        assert_eq!(std::fs::read(new.join("profile.json")).unwrap(), b"new");
        assert_eq!(std::fs::read(old.join("profile.json")).unwrap(), b"old", "the older folder is left alone");
    }

    #[test]
    fn a_fresh_install_makes_nothing() {
        let t = spaced_tempdir();
        let (old, new) = (t.path().join(crate::settings::LEGACY_DIR), t.path().join(crate::settings::DIR));
        assert_eq!(adopt_legacy_data_dir(&old, &new), new);
        assert!(!old.exists() && !new.exists());
    }

    #[test]
    fn a_folder_that_cant_move_is_used_where_it_is() {
        let t = spaced_tempdir();
        let (old, new) = (t.path().join(crate::settings::LEGACY_DIR), t.path().join("missing parent").join(crate::settings::DIR));
        put(&old, "profile.json", b"p");
        assert_eq!(adopt_legacy_data_dir(&old, &new), old);
        assert!(old.join("profile.json").is_file());
    }

    #[test]
    fn copy_tree_keeps_what_is_newer_in_the_target() {
        let t = spaced_tempdir();
        let (from, to) = (t.path().join("from"), t.path().join("to"));
        put(&from, "a/one.txt", b"from one");
        put(&from, "two.txt", b"from two");
        put(&to, "two.txt", b"to two");
        // `to` was written after `from`.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        std::fs::File::options().write(true).open(to.join("two.txt")).unwrap().set_modified(later).unwrap();
        copy_tree(&from, &to, true).unwrap();
        assert_eq!(std::fs::read(to.join("a/one.txt")).unwrap(), b"from one", "a missing file is copied");
        assert_eq!(std::fs::read(to.join("two.txt")).unwrap(), b"to two", "a newer file stays");
        copy_tree(&from, &to, false).unwrap();
        assert_eq!(std::fs::read(to.join("two.txt")).unwrap(), b"from two");
    }
}
