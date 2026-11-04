//! Save backups taken before every deploy.

use std::path::{Path, PathBuf};

use crate::Result;
use crate::game::GameInstall;

/// Backs up the game's save folder and every Steam Cloud save folder. The cloud files go in a
/// `steam-<account>` subfolder of the backup, because the same names exist per account.
pub fn backup_game_saves(game: &GameInstall, backups_dir: &Path, keep: usize) -> Result<Option<PathBuf>> {
    let mut sources = vec![(String::new(), game.saves_dir.clone())];
    for remote in &game.cloud_saves {
        // <steam>/userdata/<account>/394510/remote
        let account = remote.ancestors().nth(2).and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        sources.push((format!("steam-{account}"), remote.clone()));
    }
    backup_saves(&sources, backups_dir, keep)
}

/// Whether every file under `dir` is under `copy`, with the same bytes.
fn all_in(dir: &Path, copy: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    entries.flatten().all(|e| {
        let there = copy.join(e.file_name());
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            all_in(&e.path(), &there)
        } else {
            matches!((std::fs::read(e.path()), std::fs::read(there)), (Ok(a), Ok(b)) if a == b)
        }
    })
}

/// Copies each `(subfolder, folder)` that exists into `backups_dir/YYYYMMDD-HHMMSS` (adding `-2`,
/// `-3`... on a clash), an empty subfolder meaning the backup itself. Saves that haven't changed since
/// the newest backup make no new one. It keeps `keep` backups: the oldest, which may be the last save
/// from before any mod, and the newest after it. Ten deploys in a row must not cost that one.
pub fn backup_saves(sources: &[(String, PathBuf)], backups_dir: &Path, keep: usize) -> Result<Option<PathBuf>> {
    if !sources.iter().any(|(_, dir)| dir.is_dir()) {
        return Ok(None);
    }
    std::fs::create_dir_all(backups_dir)?;
    if let Some(newest) = list_backups(backups_dir).into_iter().next()
        && sources.iter().filter(|(_, dir)| dir.is_dir()).all(|(name, dir)| all_in(dir, &newest.join(name)))
    {
        return Ok(Some(newest));
    }
    let stamp = stamp();
    // Number above every existing backup of this second: reusing a pruned name would sort the new copy as the oldest.
    let highest = std::fs::read_dir(backups_dir)?
        .filter_map(|e| e.ok().and_then(|e| order_key(&e.path())))
        .filter(|(s, _)| *s == stamp)
        .map(|(_, n)| n)
        .max();
    let target = match highest {
        None => backups_dir.join(&stamp),
        Some(n) => backups_dir.join(format!("{stamp}-{}", n + 1)),
    };
    for (name, dir) in sources.iter().filter(|(_, dir)| dir.is_dir()) {
        crate::library::copy_dir(dir, &target.join(name))?;
    }
    let mut all: Vec<(String, u32, PathBuf)> = std::fs::read_dir(backups_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .filter_map(|p| order_key(&p).map(|(s, n)| (s, n, p)))
        .collect();
    // Sort by stamp, then by the numeric suffix, so "-10" comes after "-9".
    all.sort();
    let excess = all.len().saturating_sub(keep);
    for (_, _, old) in all.into_iter().skip(1).take(excess) {
        std::fs::remove_dir_all(old)?;
    }
    Ok(Some(target))
}

/// The backups in `backups_dir`, newest first.
/// The local time as a folder name: 20261104-153012.
pub fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn list_backups(backups_dir: &Path) -> Vec<PathBuf> {
    let mut all: Vec<(String, u32, PathBuf)> = std::fs::read_dir(backups_dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()).filter_map(|p| order_key(&p).map(|(s, n)| (s, n, p))).collect())
        .unwrap_or_default();
    all.sort();
    all.into_iter().rev().map(|(_, _, p)| p).collect()
}

/// (`YYYYMMDD-HHMMSS`, suffix) for a backup folder name, or None for anything else.
fn order_key(p: &Path) -> Option<(String, u32)> {
    let name = p.file_name()?.to_str()?;
    let (stamp, rest) = name.split_at_checked(15)?;
    let n = match rest {
        "" => 1,
        r => r.strip_prefix('-')?.parse().ok()?,
    };
    Some((stamp.to_string(), n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_newest_first_and_skips_other_folders() {
        let t = tempfile::tempdir().unwrap();
        for n in ["20261001-100000", "20261002-090000", "20261002-090000-2", "notes"] {
            std::fs::create_dir_all(t.path().join(n)).unwrap();
        }
        let names: Vec<String> = list_backups(t.path()).iter().map(|p| p.file_name().unwrap().to_string_lossy().into()).collect();
        assert_eq!(names, ["20261002-090000-2", "20261002-090000", "20261001-100000"]);
        assert!(list_backups(&t.path().join("missing")).is_empty());
    }
    use crate::testutil::{put, spaced_tempdir};

    #[test]
    fn backups_keep_the_first_and_the_newest_nine() {
        let t = spaced_tempdir();
        let saves = t.path().join("saves");
        let backups = t.path().join("backups");
        let sources = [(String::new(), saves.clone())];
        let made: Vec<PathBuf> = (0..12)
            .map(|i| {
                put(&saves, "profile.sav", format!("save {i}").as_bytes());
                backup_saves(&sources, &backups, 10).unwrap().unwrap()
            })
            .collect();
        let left: Vec<PathBuf> = std::fs::read_dir(&backups).unwrap().map(|e| e.unwrap().path()).collect();
        assert_eq!(left.len(), 10);
        assert!(made[0].exists(), "the save from before any mod stays");
        assert!(!made[1].exists() && !made[2].exists() && made[3].exists() && made[11].exists(), "made {made:?} left {left:?}");
        assert_eq!(std::fs::read(made[11].join("profile.sav")).unwrap(), b"save 11");
    }

    #[test]
    fn saves_that_havent_changed_make_no_new_backup() {
        let t = spaced_tempdir();
        let saves = t.path().join("saves");
        put(&saves, "profile.sav", b"save");
        let backups = t.path().join("backups");
        let sources = [(String::new(), saves.clone())];
        let first = backup_saves(&sources, &backups, 10).unwrap().unwrap();
        assert_eq!(backup_saves(&sources, &backups, 10).unwrap().unwrap(), first);
        assert_eq!(list_backups(&backups).len(), 1);
        put(&saves, "profile.sav", b"later");
        assert_ne!(backup_saves(&sources, &backups, 10).unwrap().unwrap(), first);
    }

    #[test]
    fn no_saves_folder_means_no_backup() {
        let t = spaced_tempdir();
        assert_eq!(backup_saves(&[(String::new(), t.path().join("none"))], &t.path().join("b"), 10).unwrap(), None);
    }

    #[test]
    fn the_real_save_is_backed_up_when_the_game_folder_is_empty() {
        let t = spaced_tempdir();
        std::fs::create_dir_all(t.path().join("saves")).unwrap();
        let remote = t.path().join("steam/userdata/107/394510/remote");
        put(&remote, "helldivers_save", b"the real save");
        let game = GameInstall { root: t.path().join("game"), saves_dir: t.path().join("saves"), cloud_saves: vec![remote] };
        let made = backup_game_saves(&game, &t.path().join("backups"), 10).unwrap().unwrap();
        assert_eq!(std::fs::read(made.join("steam-107").join("helldivers_save")).unwrap(), b"the real save");
    }
}
