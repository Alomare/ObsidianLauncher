//! Which Steam account the game saves under, and whether Steam tracks a Cloud file.

use std::path::{Path, PathBuf};

/// Added to an account id to get the Steam64 id.
const STEAM64_BASE: u64 = 76561197960265728;

fn is_steam64(s: &str) -> bool {
    s.len() == 17 && s.starts_with("7656") && s.bytes().all(|b| b.is_ascii_digit())
}

/// The account in `config/loginusers.vdf` to use: the one marked MostRecent, else the one with the
/// newest timestamp, else the only one listed.
pub fn from_loginusers(text: &str) -> Option<String> {
    let tokens = crate::vdf::tokens(text);
    let mut users: Vec<(String, bool, u64)> = Vec::new();
    let mut i = 0;
    // A block of a user is key, value, key, value; only a Steam id in the place of a key opens one.
    while i < tokens.len() {
        let t = &tokens[i];
        if t.eq_ignore_ascii_case("users") {
            i += 1;
            continue;
        }
        if is_steam64(t) {
            users.push((t.clone(), false, 0));
            i += 1;
            continue;
        }
        let value = tokens.get(i + 1).map(String::as_str).unwrap_or("");
        if let Some(u) = users.last_mut() {
            if t.eq_ignore_ascii_case("MostRecent") {
                u.1 = value == "1";
            } else if t.eq_ignore_ascii_case("timestamp") {
                u.2 = value.parse().unwrap_or(0);
            }
        }
        i += 2;
    }
    let best = users.iter().find(|u| u.1).or_else(|| users.iter().max_by_key(|u| u.2))?;
    best.0.parse::<u64>().ok().and_then(|id| id.checked_sub(STEAM64_BASE)).map(|a| a.to_string())
}

/// The game's id for an account: lowercase hex of the Steam64 id.
pub fn uid(account: &str) -> String {
    account.parse::<u64>().map(|a| format!("{:x}", a + STEAM64_BASE)).unwrap_or_default()
}

/// The account id of a game uid, as in `helldivers_steam_<uid>.sav`.
fn account_of_uid(uid: &str) -> Option<String> {
    u64::from_str_radix(uid, 16).ok()?.checked_sub(STEAM64_BASE).map(|a| a.to_string())
}

/// The active Steam account, from the Steam clients in `roots`. Falls back on the only account with
/// a Cloud folder, then on the local save names.
pub fn detect(roots: &[PathBuf], cloud_saves: &[PathBuf], saves_dir: &Path) -> Option<String> {
    for root in roots {
        let vdf = root.join("config").join("loginusers.vdf");
        if let Some(a) = std::fs::read_to_string(vdf).ok().as_deref().and_then(from_loginusers) {
            return Some(a);
        }
    }
    let of_cloud: Vec<String> = cloud_saves.iter().filter_map(|c| c.ancestors().nth(2)?.file_name().map(|n| n.to_string_lossy().into_owned())).collect();
    if let [only] = of_cloud.as_slice() {
        return Some(only.clone());
    }
    std::fs::read_dir(saves_dir)
        .ok()?
        .flatten()
        .find_map(|e| e.file_name().to_string_lossy().strip_prefix("helldivers_steam_")?.split(['.', '_']).next().and_then(account_of_uid))
}

/// Whether Steam tracks this file name in `remotecache.vdf` beside the `remote` folder. A file Steam
/// has no entry for stays invisible to Steam.
pub fn tracked(remote: &Path, names: &[&str]) -> bool {
    let Some(parent) = remote.parent() else { return false };
    let Ok(text) = std::fs::read_to_string(parent.join("remotecache.vdf")) else { return false };
    let tokens = crate::vdf::tokens(&text);
    names.iter().all(|n| tokens.iter().any(|t| t == n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    #[test]
    fn the_uid_is_hex_of_the_steam64_id() {
        assert_eq!(uid("107296691"), "1100001066537b3");
        assert_eq!(account_of_uid("1100001066537b3").as_deref(), Some("107296691"));
    }

    #[test]
    fn most_recent_wins_then_the_newest() {
        let two = r#""users" { "76561198067562419" { "AccountName" "a" "timestamp" "5" "MostRecent" "0" }
            "76561198000000000" { "PersonaName" "76561199999999999" "timestamp" "1" "MostRecent" "1" } }"#;
        assert_eq!(from_loginusers(two).as_deref(), Some("39734272"), "the persona name is not a user block");
        let none = r#""users" { "76561198067562419" { "timestamp" "5" } "76561198000000000" { "timestamp" "1" } }"#;
        assert_eq!(from_loginusers(none).as_deref(), Some("107296691"));
        assert_eq!(from_loginusers("\"users\" { }"), None);
    }

    #[test]
    fn detect_prefers_steam_then_the_cloud_folder_then_save_names() {
        let t = spaced_tempdir();
        put(&t.path().join("steam/config"), "loginusers.vdf", br#""users" { "76561198067562419" { "MostRecent" "1" } }"#);
        assert_eq!(detect(&[t.path().join("steam")], &[], t.path()).as_deref(), Some("107296691"));
        let cloud = t.path().join("elsewhere/userdata/55/394510/remote");
        assert_eq!(detect(&[], std::slice::from_ref(&cloud), t.path()).as_deref(), Some("55"));
        let saves = t.path().join("saves");
        put(&saves, "helldivers_steam_1100001066537b3_user.sav", b"x");
        assert_eq!(detect(&[], &[], &saves).as_deref(), Some("107296691"));
        assert_eq!(detect(&[], &[], t.path()), None);
    }

    #[test]
    fn tracked_needs_every_name_in_the_cache() {
        let t = spaced_tempdir();
        let remote = t.path().join("394510/remote");
        std::fs::create_dir_all(&remote).unwrap();
        assert!(!tracked(&remote, &["helldivers_save"]));
        put(&t.path().join("394510"), "remotecache.vdf", br#""394510" { "helldivers_save" { "size" "1" } }"#);
        assert!(tracked(&remote, &["helldivers_save"]));
        assert!(!tracked(&remote, &["helldivers_save", "helldivers_save123"]));
    }
}
