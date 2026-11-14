//! The Store: the list of mods in github.com/Alomare/ObsidianStoreIndex, read from the raw files of
//! that repository (no GitHub API call), with the pictures kept in `<data>/store`. A mod installs
//! from the newest release of the repository the list names, as a mod update does.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::updates::{GitHub, Staged, github_repo, stage_new};
use crate::{ObsidianError, Result};

/// The newest `format` of index.json this version reads.
pub const FORMAT: u32 = 1;
/// Where the index and the pictures are; `OBSIDIAN_STORE_URL` stands in for it in tests.
const BASE: &str = "https://raw.githubusercontent.com/Alomare/ObsidianStoreIndex/main/";
/// The folder in the data folder with the last index read and the pictures.
pub const DIR: &str = "store";
const INDEX: &str = "index.json";
const MAX_INDEX: u64 = 8 << 20;
const MAX_IMAGE: u64 = 4 << 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreVersion {
    pub version: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub notes: String,
}

/// An option of a mod, in English; `group` names one of the mod's `option_groups`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreOption {
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreGroup {
    pub id: String,
    pub label: String,
}

/// One mod of the index. `icon` and `banner` are paths in the index repository as read, and paths on
/// this PC once `fetch` has the pictures (None when a picture couldn't be had).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreMod {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub early: bool,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub options: u32,
    /// The options by name and tooltip, for the mod page; indexes built before 2026-11-13 lack them.
    #[serde(default)]
    pub option_list: Vec<StoreOption>,
    #[serde(default)]
    pub option_groups: Vec<StoreGroup>,
    pub repo: String,
    #[serde(default)]
    pub kit: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub released: Option<String>,
    #[serde(default)]
    pub added: Option<String>,
    #[serde(default)]
    pub versions: Vec<StoreVersion>,
}

#[derive(Debug, Deserialize)]
struct RawIndex {
    format: u32,
    mods: Vec<StoreMod>,
}

/// The Store as the UI shows it. `offline`: GitHub couldn't be reached, and this is the last list read.
#[derive(Debug, Serialize)]
pub struct StoreIndex {
    pub mods: Vec<StoreMod>,
    pub offline: bool,
}

/// A picture path the index may name: `img/<name>.jpg`, the name in lower case letters, digits and dashes.
fn image_name(path: &str) -> Option<&str> {
    let name = path.strip_prefix("img/")?;
    let stem = name.strip_suffix(".jpg")?;
    (!stem.is_empty() && stem.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')).then_some(name)
}

fn parse(text: &str) -> Result<Vec<StoreMod>> {
    let raw: RawIndex = serde_json::from_str(text)?;
    if raw.format > FORMAT {
        return Err(ObsidianError::StoreFormat);
    }
    Ok(raw.mods)
}

/// The base address, from `OBSIDIAN_STORE_URL` when set.
fn base() -> String {
    let b = std::env::var("OBSIDIAN_STORE_URL").unwrap_or_else(|_| BASE.to_string());
    if b.ends_with('/') { b } else { format!("{b}/") }
}

fn http_get(url: &str, limit: u64) -> Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent(concat!("Obsidian/", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(120)))
        .build()
        .into();
    let net = |e: &dyn std::fmt::Display| ObsidianError::UpdateNetwork(e.to_string());
    let mut res = agent.get(url).call().map_err(|e| net(&e))?;
    if !res.status().is_success() {
        return Err(net(&format!("GitHub answered {}", res.status().as_u16())));
    }
    res.body_mut().with_config().limit(limit).read_to_vec().map_err(|e| net(&e))
}

/// Reads the index from GitHub, keeps it in `<data>/store`, and gets the pictures it names.
pub fn fetch(data: &Path) -> Result<StoreIndex> {
    let base = base();
    fetch_with(&data.join(DIR), |path, limit| http_get(&format!("{base}{path}"), limit))
}

/// `fetch` with the download given: `get(path in the index repository, size limit)`.
pub fn fetch_with(dir: &Path, get: impl Fn(&str, u64) -> Result<Vec<u8>>) -> Result<StoreIndex> {
    let cached = dir.join(INDEX);
    let (mut mods, offline) = match get(INDEX, MAX_INDEX) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let mods = parse(&text)?;
            std::fs::create_dir_all(dir)?;
            std::fs::write(&cached, text)?;
            (mods, false)
        }
        Err(e @ ObsidianError::UpdateNetwork(_)) => match std::fs::read_to_string(&cached) {
            Ok(text) => (parse(&text)?, true),
            Err(_) => return Err(e),
        },
        Err(e) => return Err(e),
    };
    let img = dir.join("img");
    std::fs::create_dir_all(&img)?;
    let mut used = Vec::new();
    for m in &mut mods {
        for slot in [&mut m.icon, &mut m.banner] {
            *slot = slot.take().and_then(|p| {
                let name = image_name(&p)?;
                let local = img.join(name);
                if !local.exists() {
                    if offline {
                        return None;
                    }
                    let bytes = get(&p, MAX_IMAGE).ok()?;
                    let part = img.join(format!("{name}.part"));
                    std::fs::write(&part, bytes).ok()?;
                    std::fs::rename(&part, &local).ok()?;
                }
                used.push(name.to_string());
                Some(local.to_string_lossy().into_owned())
            });
        }
    }
    // Pictures no mod names any more go, once the list is fresh.
    if !offline {
        for e in std::fs::read_dir(&img)?.flatten() {
            if !used.iter().any(|u| *u == e.file_name().to_string_lossy()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    Ok(StoreIndex { mods, offline })
}

/// The mod `id` in the last index read, so an install goes to the repository the index names and
/// never one the window passes in.
pub fn find(data: &Path, id: &str) -> Result<StoreMod> {
    let text = std::fs::read_to_string(data.join(DIR).join(INDEX)).map_err(|_| ObsidianError::StoreUnknown(id.to_string()))?;
    parse(&text)?.into_iter().find(|m| m.id == id).ok_or_else(|| ObsidianError::StoreUnknown(id.to_string()))
}

/// Downloads the newest release of `m` and checks that it is that mod. Touches nothing in the library.
pub fn stage(github: &GitHub, m: &StoreMod) -> Result<Staged> {
    let repo = github_repo(&format!("https://github.com/{}", m.repo)).ok_or_else(|| ObsidianError::NoUpdateSource(m.id.clone()))?;
    stage_new(github, &repo, &m.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn index(format: u32, icon: &str) -> String {
        format!(
            r#"{{"format":{format},"mods":[{{"id":"a.mod","name":"A","version":"1.0.0","repo":"o/a","icon":"{icon}","banner":null,"versions":[{{"version":"1.0.0","notes":"First"}}]}}]}}"#
        )
    }

    /// A stand-in for the index repository; None for a path answers as GitHub out of reach.
    struct Fake {
        files: HashMap<String, Vec<u8>>,
        online: bool,
        asked: RefCell<Vec<String>>,
    }

    impl Fake {
        fn new(online: bool, files: &[(&str, &[u8])]) -> Self {
            Fake { files: files.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect(), online, asked: RefCell::new(Vec::new()) }
        }
        fn get(&self, path: &str, _limit: u64) -> Result<Vec<u8>> {
            self.asked.borrow_mut().push(path.to_string());
            if !self.online {
                return Err(ObsidianError::UpdateNetwork("offline".into()));
            }
            self.files.get(path).cloned().ok_or_else(|| ObsidianError::UpdateNetwork("404".into()))
        }
    }

    #[test]
    fn fetch_keeps_the_index_and_the_pictures() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("img")).unwrap();
        std::fs::write(dir.path().join("img/old-icon-1.jpg"), b"old").unwrap();
        let text = index(1, "img/a-icon-2.jpg");
        let fake = Fake::new(true, &[("index.json", text.as_bytes()), ("img/a-icon-2.jpg", b"jpeg")]);
        let got = fetch_with(dir.path(), |p, l| fake.get(p, l)).unwrap();
        assert!(!got.offline);
        let icon = got.mods[0].icon.clone().unwrap();
        assert_eq!(std::fs::read(&icon).unwrap(), b"jpeg");
        assert!(!dir.path().join("img/old-icon-1.jpg").exists(), "a picture no mod names is removed");
        assert_eq!(got.mods[0].versions[0].notes, "First");

        // A second read finds the picture on this PC and doesn't download it again.
        fake.asked.borrow_mut().clear();
        fetch_with(dir.path(), |p, l| fake.get(p, l)).unwrap();
        assert_eq!(*fake.asked.borrow(), vec!["index.json".to_string()]);
    }

    #[test]
    fn offline_shows_the_last_index() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Fake::new(false, &[]);
        assert!(matches!(fetch_with(&dir.path().join(DIR), |p, l| fake.get(p, l)), Err(ObsidianError::UpdateNetwork(_))));

        let text = index(1, "img/a-icon-2.jpg");
        let online = Fake::new(true, &[("index.json", text.as_bytes()), ("img/a-icon-2.jpg", b"jpeg")]);
        fetch_with(&dir.path().join(DIR), |p, l| online.get(p, l)).unwrap();
        let got = fetch_with(&dir.path().join(DIR), |p, l| fake.get(p, l)).unwrap();
        assert!(got.offline);
        assert_eq!(got.mods[0].id, "a.mod");
        assert!(got.mods[0].icon.is_some(), "a picture already here still shows");
        assert_eq!(find(dir.path(), "a.mod").unwrap().repo, "o/a");
        assert!(matches!(find(dir.path(), "b.mod"), Err(ObsidianError::StoreUnknown(_))));
    }

    #[test]
    fn options_read_with_their_groups_and_default_to_none() {
        let old = parse(&index(1, "img/a.jpg")).unwrap();
        assert!(old[0].option_list.is_empty() && old[0].option_groups.is_empty());
        let text = r#"{"format":1,"mods":[{"id":"a.mod","name":"A","version":"1.0.0","repo":"o/a","options":2,
            "option_list":[{"label":"Lives","description":"Per mission","group":null},{"label":"Price","group":"shop"}],
            "option_groups":[{"id":"shop","label":"Shop"}]}]}"#;
        let m = &parse(text).unwrap()[0];
        assert_eq!(m.option_list[0], StoreOption { label: "Lives".into(), description: "Per mission".into(), group: None });
        assert_eq!(m.option_list[1].group.as_deref(), Some("shop"));
        assert_eq!(m.option_groups, vec![StoreGroup { id: "shop".into(), label: "Shop".into() }]);
    }

    #[test]
    fn a_newer_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let text = index(FORMAT + 1, "img/a.jpg");
        let fake = Fake::new(true, &[("index.json", text.as_bytes())]);
        assert!(matches!(fetch_with(dir.path(), |p, l| fake.get(p, l)), Err(ObsidianError::StoreFormat)));
    }

    #[test]
    fn picture_paths_stay_in_img() {
        for bad in ["../a.jpg", "img/../a.jpg", "img/a.png", "img/A.jpg", "img/.jpg", "img/a/b.jpg", "/img/a.jpg", "img\\a.jpg"] {
            assert_eq!(image_name(bad), None, "{bad}");
        }
        assert_eq!(image_name("img/alomare-betterhud-icon-f8848148c9.jpg"), Some("alomare-betterhud-icon-f8848148c9.jpg"));

        let dir = tempfile::tempdir().unwrap();
        let text = index(1, "../evil.jpg");
        let fake = Fake::new(true, &[("index.json", text.as_bytes()), ("../evil.jpg", b"x")]);
        let got = fetch_with(dir.path(), |p, l| fake.get(p, l)).unwrap();
        assert_eq!(got.mods[0].icon, None);
        assert!(!fake.asked.borrow().iter().any(|p| p.contains("evil")));
    }
}
