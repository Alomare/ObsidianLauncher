//! The Enhanced War Client mod, which every player of an Enhanced war runs: the client side of the
//! custom events, orders and subfactions an Enhanced war adds to the Vanilla Galactic War.
//! Creating, selecting or joining an Enhanced war puts it in the library from the newest GitHub
//! release when it isn't there, and switches it on in the active profile, with the mods it requires.

use std::path::Path;

use serde::Serialize;

use super::ENHANCED_MOD;
use crate::Result;
use crate::library::{Library, Profile};
use crate::updates::{GitHub, Staged, github_repo, stage_new};

/// Where the releases of the mod are.
pub const ENHANCED_SOURCE: &str = "https://github.com/Alomare/HD1-EnhancedWarClient";

/// Whether the library lacks the mod, so `fetch` has to run first.
pub fn missing(data: &Path) -> Result<bool> {
    Ok(!Library::open(data)?.list()?.iter().any(|m| m.id == ENHANCED_MOD))
}

/// Downloads the newest release of the mod. This is the slow part and touches nothing in the
/// library, so it can run outside the app turn.
pub fn fetch(github: &GitHub) -> Result<Staged> {
    let repo = github_repo(ENHANCED_SOURCE).expect("a GitHub address");
    stage_new(github, &repo, ENHANCED_MOD)
}

/// What `switch_on` did.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Switched {
    /// The version installed now, when the mod came from GitHub.
    pub installed: Option<String>,
    /// Whether the mod was off in the active profile and is on now.
    pub enabled: bool,
}

/// Installs the downloaded mod, when there is one, and switches the mod on in the active profile
/// with what it requires. A mod already in the library stays as it is: updates are for the player.
pub fn switch_on(data: &Path, staged: Option<&Staged>) -> Result<Switched> {
    let lib = Library::open(data)?;
    let mut installed = None;
    if let Some(s) = staged
        && !lib.list()?.iter().any(|m| m.id == ENHANCED_MOD)
    {
        s.install(&lib)?;
        installed = Some(s.manifest.version.to_string());
    }
    let mut p = Profile::load(data)?;
    p.sync(&lib)?;
    let was_on = p.mods.iter().any(|m| m.id == ENHANCED_MOD && m.enabled);
    p.switch(&lib, ENHANCED_MOD, true)?;
    p.save(data)?;
    Ok(Switched { installed, enabled: !was_on })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObsidianError;
    use crate::testutil::{put, spaced_tempdir};

    fn stage_mod(root: &Path, id: &str, requires: &str) -> std::path::PathBuf {
        let d = root.join(format!("stage-{id}"));
        let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"0.1.0","name":{{"en":"{id}"}},"requires":[{requires}]}}"#);
        put(&d, "obsidian.json", json.as_bytes());
        d
    }

    fn on(data: &Path, id: &str) -> bool {
        Profile::load(data).unwrap().mods.iter().any(|m| m.id == id && m.enabled)
    }

    #[test]
    fn the_client_goes_on_with_the_mods_it_requires() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        let lib = Library::open(&data).unwrap();
        assert!(missing(&data).unwrap());
        lib.install(&stage_mod(t.path(), "obsidian.api", "")).unwrap();
        lib.install(&stage_mod(t.path(), ENHANCED_MOD, r#""obsidian.api""#)).unwrap();
        assert!(!missing(&data).unwrap());

        let s = switch_on(&data, None).unwrap();
        assert_eq!(s, Switched { installed: None, enabled: true });
        assert!(on(&data, ENHANCED_MOD) && on(&data, "obsidian.api"));
        assert_eq!(switch_on(&data, None).unwrap(), Switched { installed: None, enabled: false }, "already on");
    }

    #[test]
    fn a_missing_client_without_a_download_is_an_error() {
        let t = spaced_tempdir();
        let data = t.path().join("data");
        assert!(matches!(switch_on(&data, None), Err(ObsidianError::ModMissing(_))));
    }

    #[test]
    fn the_source_is_a_repository_obsidian_reads() {
        assert_eq!(github_repo(ENHANCED_SOURCE).unwrap().to_string(), "Alomare/HD1-EnhancedWarClient");
    }
}
