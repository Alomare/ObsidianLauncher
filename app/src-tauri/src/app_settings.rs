//! The app's own settings (appearance, language, updates, game folder) in `<data>/app.json`.
//! They belong to the app, not to a profile. Window size and position are kept by the
//! window-state plugin.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE: &str = "app.json";
pub const SCALES: [u32; 4] = [90, 100, 110, 125];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    /// Also what a theme this version doesn't have becomes, so one setting can't lose the others.
    #[default]
    #[serde(other)]
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSize {
    Small,
    #[default]
    Normal,
    Large,
}

/// What a new version of Obsidian found at startup does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMode {
    Manual,
    #[default]
    Prompt,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub theme: Theme,
    /// UI scale in percent, one of `SCALES`.
    pub scale: u32,
    pub text: TextSize,
    /// Animations; `None` follows the Windows reduced-motion setting.
    pub motion: Option<bool>,
    /// UI language; `None` follows the system language.
    pub language: Option<String>,
    pub updates: UpdateMode,
    /// The game folder chosen in Settings; `None` searches the Steam libraries.
    pub game_dir: Option<PathBuf>,
    /// Play asks first when no mods are deployed, as the game would start vanilla.
    pub ask_vanilla_play: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: Theme::Default,
            scale: 100,
            text: TextSize::Normal,
            motion: None,
            language: None,
            updates: UpdateMode::Prompt,
            game_dir: None,
            ask_vanilla_play: true,
        }
    }
}

impl AppSettings {
    /// The saved settings; a missing or damaged file gives the defaults.
    pub fn load(data: &Path) -> AppSettings {
        let mut s: AppSettings = std::fs::read_to_string(data.join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        if !SCALES.contains(&s.scale) {
            s.scale = 100;
        }
        s
    }

    pub fn save(&self, data: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(data)?;
        obsidian_core::paths::write_atomic(&data.join(FILE), (serde_json::to_string_pretty(self)? + "\n").as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing_or_damaged() {
        let t = tempfile::tempdir().unwrap();
        let d = AppSettings::load(t.path());
        assert_eq!(d, AppSettings::default());
        assert_eq!((d.updates, d.scale), (UpdateMode::Prompt, 100));
        std::fs::write(t.path().join(FILE), "{ not json").unwrap();
        assert_eq!(AppSettings::load(t.path()), AppSettings::default());
    }

    #[test]
    fn round_trip_and_partial_files() {
        let t = tempfile::tempdir().unwrap();
        let s = AppSettings { theme: Theme::Light, scale: 125, motion: Some(false), language: Some("pt-BR".into()), ..Default::default() };
        s.save(t.path()).unwrap();
        assert_eq!(AppSettings::load(t.path()), s);
        std::fs::write(t.path().join(FILE), r#"{"text":"large"}"#).unwrap();
        assert_eq!(AppSettings::load(t.path()).theme, Theme::Default, "missing keys take defaults");
        std::fs::write(t.path().join(FILE), r#"{"theme":"light","nav":"left"}"#).unwrap();
        assert_eq!(AppSettings::load(t.path()).theme, Theme::Light, "keys from older versions are ignored");
    }

    #[test]
    fn the_old_dark_theme_becomes_default_and_keeps_the_rest() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join(FILE), r#"{"theme":"dark","scale":125,"language":"pt-BR"}"#).unwrap();
        let s = AppSettings::load(t.path());
        assert_eq!((s.theme, s.scale, s.language.as_deref()), (Theme::Default, 125, Some("pt-BR")));
    }

    #[test]
    fn the_question_before_a_vanilla_play_is_on_until_turned_off() {
        let t = tempfile::tempdir().unwrap();
        assert!(AppSettings::load(t.path()).ask_vanilla_play, "on in a new install");
        std::fs::write(t.path().join(FILE), r#"{"theme":"light","scale":110}"#).unwrap();
        assert!(AppSettings::load(t.path()).ask_vanilla_play, "on for settings saved before it existed");
        AppSettings { ask_vanilla_play: false, ..Default::default() }.save(t.path()).unwrap();
        assert!(std::fs::read_to_string(t.path().join(FILE)).unwrap().contains(r#""askVanillaPlay": false"#), "saved under the name the app sends");
        assert!(!AppSettings::load(t.path()).ask_vanilla_play, "off stays off");
    }

    #[test]
    fn scale_outside_the_steps_becomes_100() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join(FILE), r#"{"scale":300}"#).unwrap();
        assert_eq!(AppSettings::load(t.path()).scale, 100);
    }
}
