//! The mod-set hash (spec 6.2), the tags of the Obsidian matchmaking pools (spec 6.3), and the
//! hash of each mod every player has to run.

use sha2::{Digest, Sha256};

/// A session-scope mod as the hash sees the mod: id, version and option values.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionMod {
    pub id: String,
    pub version: semver::Version,
    /// (option id, value, marked `"scope": "client"`)
    pub options: Vec<(String, serde_json::Value, bool)>,
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// `id@version`, followed by the sorted `;option=value` pairs of every option not marked client scope.
fn line(m: &SessionMod) -> String {
    let mut opts: Vec<String> = m.options.iter().filter(|(_, _, client)| !client).map(|(id, v, _)| format!(";{id}={v}")).collect();
    opts.sort();
    format!("{}@{}{}", m.id, m.version, opts.concat())
}

/// SHA-256 over the sorted lines of the mods.
pub fn modset_hash(mods: &[SessionMod]) -> String {
    let mut lines: Vec<String> = mods.iter().map(line).collect();
    lines.sort();
    sha256_hex(&lines.join("\n"))
}

/// What another player has to run for a mod marked `every_player` to run: 16 hex digits of the
/// SHA-256 of the mod's line, so another version or another shared option value is another mod.
pub fn every_hash(m: &SessionMod) -> String {
    sha256_hex(&line(m))[..16].to_string()
}

/// The tag appended to the vanilla build hash for players who run session content or allow other
/// players' session mods: 12 lowercase hex digits, the same for everyone.
pub fn open_pool_tag() -> String {
    sha256_hex("obsidian:open:1")[..12].to_string()
}

/// The tag in place of the open one for players who run mods marked `every_player`, from each
/// mod's id and `every_hash`: they find only the players who run the same ones.
pub fn every_pool_tag<'a>(every: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut pairs: Vec<String> = every.into_iter().map(|(id, hash)| format!("{id}:{hash}")).collect();
    pairs.sort();
    sha256_hex(&format!("obsidian:every:1\n{}", pairs.join("\n")))[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn m(id: &str, options: Vec<(&str, serde_json::Value, bool)>) -> SessionMod {
        SessionMod {
            id: id.into(),
            version: semver::Version::new(1, 0, 0),
            options: options.into_iter().map(|(a, b, c)| (a.to_string(), b, c)).collect(),
        }
    }

    #[test]
    fn modset_hash_ignores_order() {
        let a = m("a", vec![("x", json!(1), false), ("y", json!(true), false)]);
        let b = m("b", vec![]);
        let a2 = m("a", vec![("y", json!(true), false), ("x", json!(1), false)]);
        assert_eq!(modset_hash(&[a, b.clone()]), modset_hash(&[b, a2]));
    }

    #[test]
    fn session_option_changes_the_hash() {
        assert_ne!(modset_hash(&[m("a", vec![("x", json!(1), false)])]), modset_hash(&[m("a", vec![("x", json!(2), false)])]));
    }

    #[test]
    fn client_option_does_not_change_the_hash() {
        assert_eq!(modset_hash(&[m("a", vec![("cam", json!(1), true)])]), modset_hash(&[m("a", vec![("cam", json!(9), true)])]));
    }

    #[test]
    fn every_hash_follows_the_version_and_shared_options() {
        let a = m("a", vec![("x", json!(1), false), ("cam", json!(1), true)]);
        assert_eq!(every_hash(&a).len(), 16);
        assert_ne!(every_hash(&a), every_hash(&m("a", vec![("x", json!(2), false), ("cam", json!(1), true)])));
        assert_eq!(every_hash(&a), every_hash(&m("a", vec![("x", json!(1), false), ("cam", json!(9), true)])));
        let newer = SessionMod { version: semver::Version::new(1, 0, 1), ..a.clone() };
        assert_ne!(every_hash(&a), every_hash(&newer));
    }

    #[test]
    fn every_pool_tag_ignores_order_and_leaves_the_open_pool() {
        let tag = every_pool_tag([("a", "11"), ("b", "22")]);
        assert_eq!(tag, every_pool_tag([("b", "22"), ("a", "11")]));
        assert_ne!(tag, every_pool_tag([("a", "11")]));
        assert_ne!(tag, open_pool_tag());
        assert_eq!(tag.len(), 12);
    }

    #[test]
    fn pool_tag_is_12_lowercase_hex() {
        let t = open_pool_tag();
        assert_eq!(t.len(), 12);
        assert!(t.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    }
}
