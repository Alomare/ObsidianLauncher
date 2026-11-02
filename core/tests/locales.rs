//! The catalogs in `locales/` hold two groups of keys with the same `%s` placeholders as `en.json`:
//! loader keys in the loader languages, and `app.` keys in the app languages.

use std::collections::BTreeMap;
use std::path::PathBuf;

const LOADER_LANGUAGES: [&str; 4] = ["en", "fr", "es", "pt-BR"];
const APP_LANGUAGES: [&str; 3] = ["en", "pt-BR", "zh-CN"];
const LOADER_KEYS: [&str; 6] = ["mod_failed", "build_unknown", "session_not_cleared", "uploads_blocked", "every_player_off", "proving_grounds_blocked"];

fn catalog(lang: &str) -> BTreeMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../locales").join(format!("{lang}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn group(lang: &str, app: bool) -> BTreeMap<String, String> {
    catalog(lang).into_iter().filter(|(k, _)| k.starts_with("app.") == app).collect()
}

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
            let other = group(lang, app);
            assert_eq!(other.keys().collect::<Vec<_>>(), en.keys().collect::<Vec<_>>(), "{lang} keys (app: {app})");
            for (key, text) in &en {
                assert_eq!(other[key].matches("%s").count(), text.matches("%s").count(), "{lang} {key} placeholders");
            }
        }
    }
}
