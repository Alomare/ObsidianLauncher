//! Profiles on GitHub Gists: a link to share one profile, and a private gist that keeps the profiles,
//! the appearance settings and the reports in step between PCs.
//!
//! A gist holds text only, so mods travel as their id, version and the repository they update from,
//! and install from that repository's newest release on the other side. A mod with no repository
//! can't come along; the import names it.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::github::{self, Client, Report};
use crate::library::{Library, Profile};
use crate::manifest::load_manifest;
use crate::updates::{GitHub, github_repo, source_of, stage_new};
use crate::{ObsidianError, Result};

const SHARE_FILE: &str = "obsidian-profile.json";
const SYNC_FILE: &str = "obsidian-sync.json";

/// A mod as a gist names it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModRef {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub version: String,
    /// `https://github.com/<owner>/<repo>`, or None for a mod that names no repository.
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Shared {
    obsidian_profile: u32,
    name: String,
    profile: Profile,
    mods: Vec<ModRef>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Synced {
    obsidian_sync: u32,
    updated: String,
    profiles: BTreeMap<String, Profile>,
    mods: Vec<ModRef>,
    #[serde(default)]
    settings: Value,
    #[serde(default)]
    reports: Vec<Report>,
}

fn refs(lib: &Library, ids: &[String]) -> Result<Vec<ModRef>> {
    let mut out = Vec::new();
    for id in ids {
        let Ok(m) = load_manifest(&lib.mod_dir(id)) else { continue };
        let name = m.name.get("en").or_else(|| m.name.values().next()).cloned().unwrap_or_default();
        out.push(ModRef { id: m.id.clone(), name, version: m.version.to_string(), source: source_of(&m).map(|r| format!("https://github.com/{r}")) });
    }
    Ok(out)
}

/// The mods of `want` the library lacks or has older, installed from their repositories. Returns the
/// ids installed, kept and left out (no repository, or the download failed).
fn fetch_mods(lib: &Library, releases: &GitHub, want: &[ModRef]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (mut installed, mut kept, mut missing) = (Vec::new(), Vec::new(), Vec::new());
    for r in want {
        let wanted = semver::Version::parse(&r.version).ok();
        if let Ok(have) = load_manifest(&lib.mod_dir(&r.id))
            && wanted.as_ref().is_none_or(|w| have.version >= *w)
        {
            kept.push(r.id.clone());
            continue;
        }
        let got = r
            .source
            .as_deref()
            .and_then(github_repo)
            .ok_or(ObsidianError::NoUpdateSource(r.id.clone()))
            .and_then(|repo| stage_new(releases, &repo, &r.id))
            .and_then(|s| s.install(lib).map(|_| ()));
        match got {
            Ok(()) => installed.push(r.id.clone()),
            // An older copy already here still beats nothing.
            Err(_) if lib.mod_dir(&r.id).is_dir() => kept.push(r.id.clone()),
            Err(_) => missing.push(if r.name.is_empty() { r.id.clone() } else { r.name.clone() }),
        }
    }
    (installed, kept, missing)
}

/// Shares profile `name` as a gist, public (listed on the player's GitHub page) or secret (only
/// those with the link). Disabled mods stay home, as with a profile ZIP. Returns the link.
pub fn share_profile(data: &Path, c: &Client, name: &str, public: bool) -> Result<String> {
    let lib = Library::open(data)?;
    let mut profile = Profile::load_named(data, name)?;
    profile.sync(&lib)?;
    profile.mods.retain(|m| m.enabled);
    profile.options.retain(|id, _| profile.mods.iter().any(|m| &m.id == id));
    profile.keys.retain(|id, _| profile.mods.iter().any(|m| &m.id == id));
    profile.block_public = false;
    let ids: Vec<String> = profile.mods.iter().map(|m| m.id.clone()).collect();
    let shared = Shared { obsidian_profile: 1, name: name.to_string(), mods: refs(&lib, &ids)?, profile };
    let mut files = BTreeMap::new();
    files.insert(SHARE_FILE.to_string(), serde_json::to_string_pretty(&shared)? + "\n");
    let (_, url) = github::create_gist(c, &format!("Obsidian Launcher profile: {name}"), public, &files)?;
    Ok(url)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedImport {
    pub name: String,
    pub installed: Vec<String>,
    pub kept: Vec<String>,
    /// Mods the profile names that couldn't be installed, by name.
    pub missing: Vec<String>,
    pub guardrails: Vec<String>,
}

/// Adds the profile behind a gist link as a new profile, installing its mods from their
/// repositories. Works signed out too, since a shared gist is readable by anyone with the link.
pub fn import_shared(data: &Path, c: &Client, releases: &GitHub, link: &str) -> Result<SharedImport> {
    let id = github::gist_id(link).ok_or_else(|| ObsidianError::NothingToImport(link.to_string()))?;
    let (files, _) = github::read_gist(c, &id)?.ok_or_else(|| ObsidianError::NothingToImport(link.to_string()))?;
    let text = files.get(SHARE_FILE).ok_or_else(|| ObsidianError::NothingToImport(link.to_string()))?;
    let shared: Shared = serde_json::from_str(text)?;
    let lib = Library::open(data)?;
    let (installed, kept, missing) = fetch_mods(&lib, releases, &shared.mods);
    let taken = crate::profiles::list(data)?;
    let base = if crate::profiles::check_name(&shared.name).is_ok() { shared.name.clone() } else { "Shared".to_string() };
    let free = |n: &str| !taken.iter().any(|t| t.to_lowercase() == n.to_lowercase());
    let name = (1..).map(|i| if i == 1 { base.clone() } else { format!("{base} ({i})") }).find(|n| free(n) && n.chars().count() <= 40).unwrap_or_else(|| "Shared".into());
    crate::profiles::create(data, &name, None)?;
    let mut profile = shared.profile;
    profile.sync(&lib)?;
    profile.block_public = false;
    let mut guardrails = Vec::new();
    if profile.block_achievements {
        guardrails.push("block_achievements".to_string());
    }
    if profile.freeze_progress {
        guardrails.push("freeze_progress".to_string());
    }
    profile.save_named(data, &name)?;
    Ok(SharedImport { name, installed, kept, missing, guardrails })
}

/// Where sync stands: the gist's link and when it last changed, if there is one.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub url: Option<String>,
    pub updated: Option<String>,
}

fn gist_url(id: &str) -> String {
    format!("https://gist.github.com/{id}")
}

pub fn sync_state(data: &Path, c: &Client) -> Result<SyncState> {
    let Some(id) = github::find_sync_gist(data, c)? else { return Ok(SyncState::default()) };
    let read = github::read_gist(c, &id)?;
    let updated = read.and_then(|(files, at)| files.get(SYNC_FILE).and_then(|t| serde_json::from_str::<Synced>(t).ok()).map(|s| s.updated).or(Some(at)));
    Ok(SyncState { url: Some(gist_url(&id)), updated })
}

/// Writes every profile, the mods they name, `settings` and the reports to the sync gist (a secret
/// one, made the first time).
pub fn upload(data: &Path, c: &Client, settings: Value) -> Result<SyncState> {
    let lib = Library::open(data)?;
    let mut profiles = BTreeMap::new();
    let mut ids = Vec::new();
    for name in crate::profiles::list(data)? {
        let mut p = Profile::load_named(data, &name)?;
        p.sync(&lib)?;
        for m in &p.mods {
            if !ids.contains(&m.id) {
                ids.push(m.id.clone());
            }
        }
        profiles.insert(name, p);
    }
    let updated = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let synced = Synced { obsidian_sync: 1, updated: updated.clone(), profiles, mods: refs(&lib, &ids)?, settings, reports: github::reports(data) };
    let mut files = BTreeMap::new();
    files.insert(SYNC_FILE.to_string(), serde_json::to_string_pretty(&synced)? + "\n");
    let id = match github::find_sync_gist(data, c)? {
        Some(id) => {
            github::update_gist(c, &id, &files)?;
            id
        }
        None => {
            let (id, _) = github::create_gist(c, github::SYNC_GIST, false, &files)?;
            github::remember_sync_gist(data, &id)?;
            id
        }
    };
    Ok(SyncState { url: Some(gist_url(&id)), updated: Some(updated) })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloaded {
    pub profiles: Vec<String>,
    pub installed: Vec<String>,
    pub missing: Vec<String>,
    /// The settings as uploaded, for the app to take what belongs to every PC.
    pub settings: Value,
    pub updated: String,
}

/// Brings the sync gist to this PC: profiles of the same name are replaced, others are added, none
/// are removed; missing mods install from their repositories; reports are merged.
pub fn download(data: &Path, c: &Client, releases: &GitHub) -> Result<Downloaded> {
    let id = github::find_sync_gist(data, c)?.ok_or_else(|| ObsidianError::GitHub("nothing was uploaded from another PC yet".into()))?;
    let (files, _) = github::read_gist(c, &id)?.ok_or_else(|| ObsidianError::GitHub("the sync gist is gone".into()))?;
    let synced: Synced = serde_json::from_str(files.get(SYNC_FILE).ok_or_else(|| ObsidianError::GitHub("the sync gist has no profiles".into()))?)?;
    if synced.obsidian_sync > 1 {
        return Err(ObsidianError::GitHub("the sync gist was written by a newer Obsidian".into()));
    }
    let lib = Library::open(data)?;
    let (installed, _, missing) = fetch_mods(&lib, releases, &synced.mods);
    let mut names = Vec::new();
    for (name, mut p) in synced.profiles {
        if crate::profiles::check_name(&name).is_err() {
            continue;
        }
        // The same profile in another case is the same file on Windows: write over that one.
        let target = crate::profiles::find(data, &name)?.unwrap_or(name);
        p.sync(&lib)?;
        p.save_named(data, &target)?;
        names.push(target);
    }
    let mut reports = github::reports(data);
    for r in synced.reports {
        if !reports.iter().any(|x| x.repo == r.repo && x.number == r.number) {
            reports.push(r);
        }
    }
    reports.sort_by(|a, b| b.created.cmp(&a.created));
    crate::paths::write_atomic(&data.join("github-reports.json"), &serde_json::to_vec_pretty(&reports)?)?;
    Ok(Downloaded { profiles: names, installed, missing, settings: synced.settings, updated: synced.updated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    fn add(lib: &Library, root: &Path, id: &str, version: &str, source: Option<&str>) {
        let d = root.join(format!("stage-{id}-{version}"));
        let src = source.map(|s| format!(r#","source":"{s}""#)).unwrap_or_default();
        put(&d, "obsidian.json", format!(r#"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}}{src}}}"#).as_bytes());
        lib.install(&d).unwrap();
    }

    #[test]
    fn mods_travel_by_id_version_and_repository() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("home")).unwrap();
        add(&lib, t.path(), "a.mod", "1.2.0", Some("https://github.com/me/a"));
        add(&lib, t.path(), "b.mod", "0.1.0", None);
        let r = refs(&lib, &["a.mod".into(), "b.mod".into(), "gone".into()]).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], ModRef { id: "a.mod".into(), name: "a.mod".into(), version: "1.2.0".into(), source: Some("https://github.com/me/a".into()) });
        assert_eq!(r[1].source, None);

        // On the other PC: a newer copy is kept, one with no repository can't come.
        let other = Library::open(&t.path().join("away")).unwrap();
        add(&other, t.path(), "a.mod", "1.3.0", None);
        let (installed, kept, missing) = fetch_mods(&other, &GitHub::at("http://127.0.0.1:9"), &r);
        assert!(installed.is_empty());
        assert_eq!(kept, vec!["a.mod".to_string()]);
        assert_eq!(missing, vec!["b.mod".to_string()]);
    }
}
