//! Mod updates: a mod whose `source` is a GitHub repository updates from that repository's latest
//! release. Nothing here runs unless the player asks, and a release can only replace the mod it
//! belongs to, with a higher version.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::library::{InstallOutcome, Library};
use crate::manifest::{Manifest, load_manifest};
use crate::{ObsidianError, Result};

/// The largest file Obsidian downloads, so a bad release can't fill the disk.
const MAX_DOWNLOAD: u64 = 1 << 30;

#[derive(Debug, Clone, PartialEq)]
pub struct Repo {
    owner: String,
    name: String,
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// The repository in a `source` address: `https://github.com/<owner>/<repo>`, ignoring what follows.
/// Anything else, a plain `http` address included, is not one Obsidian updates from.
pub fn github_repo(source: &str) -> Option<Repo> {
    let rest = source.trim().strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let name = name.strip_suffix(".git").unwrap_or(name);
    let fits = |s: &str| !s.is_empty() && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
    (fits(owner) && fits(name)).then(|| Repo { owner: owner.to_string(), name: name.to_string() })
}

/// The repository a mod updates from.
pub fn source_of(m: &Manifest) -> Option<Repo> {
    github_repo(m.extra.get("source")?.as_str()?)
}

#[derive(Debug, Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct RawRelease {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct RawNotes {
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawListed {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// A published release as the changelog shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ListedRelease {
    pub version: String,
    pub date: Option<String>,
    pub notes: String,
    pub url: String,
}

#[derive(Debug)]
pub struct Release {
    pub version: Version,
    assets: Vec<Asset>,
}

pub struct GitHub {
    api: String,
    agent: ureq::Agent,
}

impl Default for GitHub {
    /// The real GitHub, or `OBSIDIAN_GITHUB_API` (tests).
    fn default() -> Self {
        GitHub::at(std::env::var("OBSIDIAN_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".into()))
    }
}

fn network(e: impl fmt::Display) -> ObsidianError {
    ObsidianError::UpdateNetwork(e.to_string())
}

impl GitHub {
    pub fn at(api: impl Into<String>) -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .user_agent(concat!("Obsidian/", env!("CARGO_PKG_VERSION")))
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .timeout_recv_body(Some(Duration::from_secs(600)))
            .build();
        GitHub { api: api.into(), agent: config.into() }
    }

    /// The newest release of `repo`. GitHub leaves out drafts and prereleases.
    pub fn latest(&self, repo: &Repo) -> Result<Release> {
        let raw: RawRelease = serde_json::from_str(&self.release_json(repo, "/latest")?)?;
        let tag = raw.tag_name.strip_prefix(['v', 'V']).unwrap_or(&raw.tag_name);
        let version = Version::parse(tag).map_err(|_| ObsidianError::NoRelease(repo.to_string()))?;
        Ok(Release { version, assets: raw.assets })
    }

    /// The notes of one release as GitHub has them now, so an edit there counts.
    pub fn notes(&self, repo: &Repo, tag: &str) -> Result<Option<String>> {
        let raw: RawNotes = serde_json::from_str(&self.release_json(repo, &format!("/tags/{tag}"))?)?;
        Ok(raw.body.filter(|b| !b.trim().is_empty()))
    }

    /// The published releases of `repo`, newest first. A tag that isn't a version is left out.
    pub fn releases(&self, repo: &Repo) -> Result<Vec<ListedRelease>> {
        Ok(self.releases_page(repo, 1, 50)?.0)
    }

    /// One page of `releases`, counted from 1, and whether GitHub may have more after it.
    pub fn releases_page(&self, repo: &Repo, page: u32, per_page: u32) -> Result<(Vec<ListedRelease>, bool)> {
        let per_page = per_page.clamp(1, 100);
        let raw: Vec<RawListed> = serde_json::from_str(&self.release_json(repo, &format!("?per_page={per_page}&page={}", page.max(1)))?)?;
        let more = raw.len() as u32 == per_page;
        let list = raw
            .into_iter()
            .filter(|r| !r.draft && !r.prerelease)
            .filter_map(|r| {
                let version = r.tag_name.strip_prefix(['v', 'V']).unwrap_or(&r.tag_name);
                Version::parse(version).ok()?;
                Some(ListedRelease { version: version.to_string(), date: r.published_at, notes: r.body.unwrap_or_default(), url: r.html_url })
            })
            .collect();
        Ok((list, more))
    }

    fn release_json(&self, repo: &Repo, which: &str) -> Result<String> {
        let url = format!("{}/repos/{repo}/releases{which}", self.api);
        let mut res = self.agent.get(&url).header("Accept", "application/vnd.github+json").call().map_err(network)?;
        match res.status().as_u16() {
            200 => {}
            404 => return Err(ObsidianError::NoRelease(repo.to_string())),
            403 | 429 if res.headers().get("x-ratelimit-remaining").is_some_and(|v| v == "0") => return Err(ObsidianError::UpdateLimited),
            code => return Err(network(format!("GitHub answered {code}"))),
        }
        res.body_mut().read_to_string().map_err(network)
    }

    fn download(&self, url: &str, to: &Path) -> Result<()> {
        let mut res = self.agent.get(url).call().map_err(network)?;
        if !res.status().is_success() {
            return Err(network(format!("GitHub answered {}", res.status().as_u16())));
        }
        let mut body = res.body_mut().with_config().limit(MAX_DOWNLOAD).reader();
        std::io::copy(&mut body, &mut std::fs::File::create(to)?).map_err(network)?;
        Ok(())
    }
}

/// What a check found for one mod.
pub struct Checked {
    pub id: String,
    /// The newer version, or `None` when the mod is up to date.
    pub result: Result<Option<Version>>,
}

/// Asks GitHub about every mod that names a repository. One failure doesn't stop the rest, except
/// the rate limit: GitHub counts the calls made after it too.
pub fn check(github: &GitHub, mods: &[Manifest]) -> Vec<Checked> {
    let mut limited = false;
    mods.iter()
        .filter_map(|m| {
            let repo = source_of(m)?;
            let result = if limited { Err(ObsidianError::UpdateLimited) } else { github.latest(&repo).map(|r| (r.version > m.version).then_some(r.version)) };
            limited |= matches!(result, Err(ObsidianError::UpdateLimited));
            Some(Checked { id: m.id.clone(), result })
        })
        .collect()
}

/// The `.zip` of a release: the only one, or the only one with the mod id in its name.
fn pick_zip<'a>(assets: &'a [Asset], id: &str) -> Option<&'a Asset> {
    let zips: Vec<&Asset> = assets.iter().filter(|a| a.name.to_lowercase().ends_with(".zip")).collect();
    if let [one] = zips.as_slice() {
        return Some(one);
    }
    match zips.iter().filter(|a| a.name.to_lowercase().contains(id)).collect::<Vec<_>>().as_slice() {
        [one] => Some(one),
        _ => None,
    }
}

/// A checked update, unpacked and ready to go into the library.
pub struct Staged {
    _dir: tempfile::TempDir,
    root: PathBuf,
    pub manifest: Manifest,
}

impl Staged {
    pub fn install(&self, lib: &Library) -> Result<InstallOutcome> {
        lib.install(&self.root)
    }
}

/// Downloads the latest release of a mod and checks that it is this mod, in a higher version. This
/// is the slow part and touches nothing in the library, so it doesn't need to hold anything up.
pub fn stage(github: &GitHub, current: &Manifest) -> Result<Staged> {
    let mismatch = |found: String| ObsidianError::UpdateMismatch { wanted: current.id.clone(), found };
    let repo = source_of(current).ok_or_else(|| ObsidianError::NoUpdateSource(current.id.clone()))?;
    let release = github.latest(&repo)?;
    if release.version <= current.version {
        return Err(mismatch(format!("{} {}", current.id, release.version)));
    }
    let asset = pick_zip(&release.assets, &current.id).ok_or_else(|| ObsidianError::NoUpdateAsset(repo.to_string()))?;
    let dir = tempfile::tempdir()?;
    let (zip, unpacked) = (dir.path().join("update.zip"), dir.path().join("mod"));
    github.download(&asset.browser_download_url, &zip)?;
    crate::import::unzip(&zip, &unpacked)?;
    let root = crate::import::mod_folder(&unpacked)?.ok_or_else(|| ObsidianError::NoUpdateAsset(repo.to_string()))?;
    let manifest = load_manifest(&root)?;
    if manifest.id != current.id || manifest.version <= current.version {
        return Err(mismatch(format!("{} {}", manifest.id, manifest.version)));
    }
    Ok(Staged { _dir: dir, root, manifest })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::spaced_tempdir;
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex};
    use zip::write::SimpleFileOptions;

    type Routes = Arc<Mutex<Vec<(String, u16, Vec<u8>)>>>;

    /// A stand-in for GitHub: answers each path with a status and a body, and 404 for the rest.
    fn server() -> (String, Routes) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes = Routes::default();
        let served = routes.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let path = first.split(' ').nth(1).unwrap_or("").to_string();
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 2 {
                    line.clear();
                }
                let found = served.lock().unwrap().iter().find(|r| r.0 == path).map(|r| (r.1, r.2.clone()));
                let (status, body) = found.unwrap_or((404, Vec::new()));
                write!(stream, "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        (base, routes)
    }

    /// GitHub with one release of me/mod. Each asset is served at `/dl/<name>`.
    fn github(tag: &str, assets: Vec<(&str, Vec<u8>)>) -> GitHub {
        let (base, routes) = server();
        let list: Vec<String> = assets.iter().map(|(n, _)| format!(r#"{{"name":"{n}","browser_download_url":"{base}/dl/{n}"}}"#)).collect();
        let release = format!(r#"{{"tag_name":"{tag}","assets":[{}]}}"#, list.join(","));
        let mut routes = routes.lock().unwrap();
        routes.push(("/repos/me/mod/releases/latest".into(), 200, release.into_bytes()));
        routes.extend(assets.into_iter().map(|(n, b)| (format!("/dl/{n}"), 200, b)));
        GitHub::at(base)
    }

    fn manifest(id: &str, version: &str) -> String {
        format!(r#"{{"obsidian":1,"id":"{id}","version":"{version}","name":{{"en":"{id}"}},"source":"https://github.com/me/mod"}}"#)
    }

    fn installed(id: &str, version: &str) -> Manifest {
        serde_json::from_str(&manifest(id, version)).unwrap()
    }

    fn zipped(folder: &str, json: &str) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file(format!("{folder}obsidian.json"), SimpleFileOptions::default()).unwrap();
        z.write_all(json.as_bytes()).unwrap();
        z.finish().unwrap().into_inner()
    }

    fn found(g: &GitHub, version: &str) -> Option<Version> {
        check(g, &[installed("a.mod", version)]).pop().unwrap().result.unwrap()
    }

    #[test]
    fn only_github_addresses_count() {
        let repo = |s: &str| github_repo(s).map(|r| r.to_string());
        assert_eq!(repo("https://github.com/me/mod").as_deref(), Some("me/mod"));
        assert_eq!(repo(" https://github.com/me/mod.git ").as_deref(), Some("me/mod"));
        assert_eq!(repo("https://github.com/me/mod/releases/tag/v1").as_deref(), Some("me/mod"));
        for bad in ["http://github.com/me/mod", "https://gitlab.com/me/mod", "https://github.com/me", "https://github.com/../mod", "https://github.com/me/a b", "me/mod"] {
            assert_eq!(repo(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_higher_release_is_found_with_or_without_the_v() {
        assert_eq!(found(&github("v1.2.0", vec![]), "1.1.0"), Some(Version::new(1, 2, 0)));
        assert_eq!(found(&github("1.2.0", vec![]), "1.1.0"), Some(Version::new(1, 2, 0)));
    }

    #[test]
    fn the_same_or_a_lower_release_is_no_update() {
        assert_eq!(found(&github("v1.2.0", vec![]), "1.2.0"), None);
        assert_eq!(found(&github("v1.2.0", vec![]), "2.0.0"), None);
    }

    #[test]
    fn mods_without_a_repository_are_left_out_of_a_check() {
        let plain: Manifest = serde_json::from_str(r#"{"obsidian":1,"id":"b.mod","version":"1.0.0","name":{"en":"B"}}"#).unwrap();
        assert!(check(&github("v1.0.0", vec![]), &[plain]).is_empty());
    }

    #[test]
    fn the_changelog_lists_published_versions_only() {
        let (base, routes) = server();
        let list = br#"[
            {"tag_name":"v1.0.2","html_url":"u2","published_at":"2026-10-31T00:00:00Z","body":"Two"},
            {"tag_name":"v1.0.3","html_url":"u3","draft":true},
            {"tag_name":"v1.1.0-beta.1","html_url":"u4","prerelease":true},
            {"tag_name":"nightly","html_url":"u5"},
            {"tag_name":"1.0.1","html_url":"u1","body":null}
        ]"#;
        routes.lock().unwrap().push(("/repos/me/mod/releases?per_page=50&page=1".into(), 200, list.to_vec()));
        let github = GitHub::at(base);
        let found = github.releases(&github_repo("https://github.com/me/mod").unwrap()).unwrap();
        let versions: Vec<&str> = found.iter().map(|r| r.version.as_str()).collect();
        assert_eq!(versions, ["1.0.2", "1.0.1"]);
        assert_eq!(found[0].notes, "Two");
        assert_eq!(found[1].notes, "");
    }

    #[test]
    fn a_full_page_of_releases_says_more_may_follow() {
        let (base, routes) = server();
        let page = |a: &str, b: &str| format!(r#"[{{"tag_name":"{a}","html_url":"u"}},{{"tag_name":"{b}","html_url":"u"}}]"#).into_bytes();
        routes.lock().unwrap().push(("/repos/me/mod/releases?per_page=2&page=1".into(), 200, page("v1.3.0", "v1.2.0")));
        routes.lock().unwrap().push(("/repos/me/mod/releases?per_page=2&page=2".into(), 200, br#"[{"tag_name":"v1.1.0","html_url":"u"}]"#.to_vec()));
        let github = GitHub::at(base);
        let repo = github_repo("https://github.com/me/mod").unwrap();
        let (first, more) = github.releases_page(&repo, 1, 2).unwrap();
        assert_eq!((first.len(), more), (2, true));
        let (second, more) = github.releases_page(&repo, 2, 2).unwrap();
        assert_eq!((second[0].version.as_str(), more), ("1.1.0", false));
    }

    #[test]
    fn release_notes_read_as_github_has_them_now() {
        let (base, routes) = server();
        {
            let mut routes = routes.lock().unwrap();
            routes.push(("/repos/me/mod/releases/tags/v1.0.1".into(), 200, br#"{"body":"Edited on GitHub"}"#.to_vec()));
            routes.push(("/repos/me/mod/releases/tags/v1.0.2".into(), 200, br#"{"body":"  "}"#.to_vec()));
            routes.push(("/repos/me/mod/releases/tags/v1.0.3".into(), 200, br#"{"body":null}"#.to_vec()));
        }
        let github = GitHub::at(base);
        let repo = github_repo("https://github.com/me/mod").unwrap();
        assert_eq!(github.notes(&repo, "v1.0.1").unwrap().as_deref(), Some("Edited on GitHub"));
        assert_eq!(github.notes(&repo, "v1.0.2").unwrap(), None, "blank text counts as none");
        assert_eq!(github.notes(&repo, "v1.0.3").unwrap(), None);
        assert!(matches!(github.notes(&repo, "v9.9.9"), Err(ObsidianError::NoRelease(_))));
    }

    #[test]
    fn a_failed_check_names_why_and_the_next_mod_still_runs() {
        let (base, routes) = server();
        routes.lock().unwrap().push(("/repos/me/limited/releases/latest".into(), 403, Vec::new()));
        let mut limited = installed("b.mod", "1.0.0");
        limited.extra.insert("source".into(), "https://github.com/me/limited".into());
        let r = check(&GitHub::at(base), &[installed("a.mod", "1.0.0"), limited]);
        assert!(matches!(r[0].result, Err(ObsidianError::NoRelease(_))), "a repository with no releases");
        assert!(matches!(&r[1].result, Err(ObsidianError::UpdateNetwork(m)) if m.contains("403")));
    }

    #[test]
    fn a_tag_that_is_no_version_is_no_release() {
        let r = github("latest", vec![]).latest(&github_repo("https://github.com/me/mod").unwrap());
        assert!(matches!(r, Err(ObsidianError::NoRelease(_))));
    }

    #[test]
    fn an_update_replaces_the_mod_in_the_library() {
        let t = spaced_tempdir();
        let lib = Library::open(t.path()).unwrap();
        let old = t.path().join("old");
        crate::testutil::put(&old, "obsidian.json", manifest("a.mod", "1.0.0").as_bytes());
        lib.install(&old).unwrap();

        let g = github("v1.1.0", vec![("mod.zip", zipped("mod-1.1.0/", &manifest("a.mod", "1.1.0")))]);
        let staged = stage(&g, &load_manifest(&lib.mod_dir("a.mod")).unwrap()).unwrap();
        assert!(load_manifest(&lib.mod_dir("a.mod")).unwrap().version == Version::new(1, 0, 0), "staging leaves the library alone");

        let outcome = staged.install(&lib).unwrap();
        assert_eq!(outcome, InstallOutcome::Replaced { old_version: Version::new(1, 0, 0) });
        assert_eq!(load_manifest(&lib.mod_dir("a.mod")).unwrap().version, Version::new(1, 1, 0));
    }

    #[test]
    fn a_release_holding_another_mod_is_refused() {
        let g = github("v1.1.0", vec![("mod.zip", zipped("", &manifest("other.mod", "9.0.0")))]);
        let e = stage(&g, &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateMismatch { .. }), "{e}");
    }

    #[test]
    fn a_zip_older_than_its_tag_is_refused() {
        let g = github("v1.1.0", vec![("mod.zip", zipped("", &manifest("a.mod", "1.0.0")))]);
        let e = stage(&g, &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateMismatch { .. }), "{e}");
    }

    #[test]
    fn nothing_downloads_when_the_mod_is_already_current() {
        let e = stage(&github("v1.0.0", vec![]), &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateMismatch { .. }), "{e}");
    }

    #[test]
    fn a_zip_without_a_mod_is_refused() {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file("readme.txt", SimpleFileOptions::default()).unwrap();
        let g = github("v1.1.0", vec![("mod.zip", z.finish().unwrap().into_inner())]);
        assert!(matches!(stage(&g, &installed("a.mod", "1.0.0")), Err(ObsidianError::NoUpdateAsset(_))));
    }

    #[test]
    fn a_release_needs_one_zip_to_pick() {
        let none = github("v1.1.0", vec![("notes.txt", b"hi".to_vec())]);
        assert!(matches!(stage(&none, &installed("a.mod", "1.0.0")), Err(ObsidianError::NoUpdateAsset(_))));

        let zip = zipped("", &manifest("a.mod", "1.1.0"));
        let two = github("v1.1.0", vec![("one.zip", zip.clone()), ("two.zip", zip.clone())]);
        assert!(matches!(stage(&two, &installed("a.mod", "1.0.0")), Err(ObsidianError::NoUpdateAsset(_))));

        let named = github("v1.1.0", vec![("b.mod-1.1.0.zip", zipped("", &manifest("b.mod", "1.1.0"))), ("a.mod-1.1.0.zip", zip)]);
        assert_eq!(stage(&named, &installed("a.mod", "1.0.0")).unwrap().manifest.id, "a.mod");
    }

    #[test]
    fn a_missing_file_is_a_network_error() {
        let (base, routes) = server();
        let release = format!(r#"{{"tag_name":"v1.1.0","assets":[{{"name":"mod.zip","browser_download_url":"{base}/gone.zip"}}]}}"#);
        routes.lock().unwrap().push(("/repos/me/mod/releases/latest".into(), 200, release.into_bytes()));
        let e = stage(&GitHub::at(base), &installed("a.mod", "1.0.0")).err().unwrap();
        assert!(matches!(e, ObsidianError::UpdateNetwork(_)), "{e}");
    }

    #[test]
    fn a_mod_without_a_repository_says_so() {
        let plain: Manifest = serde_json::from_str(r#"{"obsidian":1,"id":"b.mod","version":"1.0.0","name":{"en":"B"}}"#).unwrap();
        assert!(matches!(stage(&github("v1.1.0", vec![]), &plain), Err(ObsidianError::NoUpdateSource(_))));
    }
}
