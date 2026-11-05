//! The catalogs in `locales/` hold two groups of keys with the same `%s` placeholders as `en.json`:
//! loader keys in the loader languages, and `app.` keys in the app languages.

use std::collections::BTreeMap;
use std::path::PathBuf;

const LOADER_LANGUAGES: [&str; 4] = ["en", "fr", "es", "pt-BR"];
const APP_LANGUAGES: [&str; 3] = ["en", "pt-BR", "zh-CN"];
const LOADER_KEYS: [&str; 14] = [
    "mod_failed",
    "build_unknown",
    "session_not_cleared",
    "uploads_blocked",
    "every_player_off",
    "proving_grounds_blocked",
    "progress_frozen",
    "achievements_blocked",
    "expansion_refused",
    "expansion_turned_away",
    "expansion_missing_here",
    "expansion_unknown_player",
    "expansion_proving_grounds",
    "campaign_not_uploaded_expansion",
];

fn catalog(lang: &str) -> BTreeMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../locales").join(format!("{lang}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn group(lang: &str, app: bool) -> BTreeMap<String, String> {
    catalog(lang).into_iter().filter(|(k, _)| k.starts_with("app.") == app).collect()
}

/// Every key another catalog holds is one `en.json` has, with the same placeholders. A key it
/// lacks shows in English: new text is translated at the last step before a release (the
/// `fully_translated` check below, and internal/release/untranslated-keys.txt).
#[test]
fn catalogs_have_the_same_keys_and_placeholders() {
    let en = catalog("en");
    for key in LOADER_KEYS {
        assert!(en.contains_key(key), "en lacks {key}");
    }
    for (app, languages) in [(false, &LOADER_LANGUAGES[..]), (true, &APP_LANGUAGES[..])] {
        let en = group("en", app);
        assert!(!en.is_empty());
        for lang in languages {
            for (key, text) in &group(lang, app) {
                let source = en.get(key).unwrap_or_else(|| panic!("{lang} has {key}, which en lacks"));
                assert_eq!(text.matches("%s").count(), source.matches("%s").count(), "{lang} {key} placeholders");
            }
        }
    }
}

/// Run before a release: `cargo test --test locales -- --ignored`.
#[test]
#[ignore = "translations wait for the last step before a release"]
fn fully_translated() {
    for (app, languages) in [(false, &LOADER_LANGUAGES[..]), (true, &APP_LANGUAGES[..])] {
        let en = group("en", app);
        for lang in languages {
            let other = group(lang, app);
            let missing: Vec<_> = en.keys().filter(|k| !other.contains_key(*k)).collect();
            assert!(missing.is_empty(), "{lang} lacks {missing:?}");
        }
    }
}
