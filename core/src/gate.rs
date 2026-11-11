//! Build gate: Obsidian deploys only to game builds whose files it knows.

use std::path::Path;

use crate::Result;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GateReport {
    pub build: Option<String>,
    pub mismatches: Vec<String>,
}

impl GateReport {
    pub fn supported(&self) -> bool {
        self.build.is_some()
    }
}

pub struct KnownBuild {
    pub name: &'static str,
    pub files: &'static [(&'static str, &'static str)],
}

pub const KNOWN_BUILDS: &[KnownBuild] = &[KnownBuild {
    name: "Steam 6001293",
    files: &[
        ("binaries/x64/helldivers.exe", "523bc8215aef7fb4f54a0c384cf9d85f875dc607897dfb42cb8a3f1e6a40f6c6"),
        ("contents/9cf71054ea26dddf", "72bebec1c173b046b03a933ef27860beddeca068a575d267fbaeb2122fffbcd4"),
        ("contents/9e13b2414b41b842", "5ded3de9994ced0ee70597115c46c3604b754897b10de689c5b127ea8b1750a8"),
    ],
}];

/// SHA-256 of a file as lowercase hex.
/// `sha256_file` of each path, in the order given, on as many cores as there are: a deploy of a
/// large mod writes several patches of hundreds of megabytes, and each is read once more for its hash.
pub fn sha256_files(paths: &[std::path::PathBuf]) -> Vec<Result<String>> {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()).min(paths.len().max(1));
    if cores < 2 {
        return paths.iter().map(|p| sha256_file(p)).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut done: Vec<(usize, Result<String>)> = std::thread::scope(|s| {
        let running: Vec<_> = (0..cores)
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(p) = paths.get(i) else { break };
                        mine.push((i, sha256_file(p)));
                    }
                    mine
                })
            })
            .collect();
        running.into_iter().flat_map(|t| t.join().expect("hashing a file doesn't panic")).collect()
    });
    done.sort_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}

pub fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut h = Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Matches the files under `root` against `known`; mismatches name the files of the closest build.
pub fn check_build(root: &Path, known: &[KnownBuild]) -> Result<GateReport> {
    let mut closest: Option<Vec<String>> = None;
    for build in known {
        let mismatches: Vec<String> = build
            .files
            .iter()
            .filter(|(rel, sha)| sha256_file(&root.join(rel)).map_or(true, |s| s != *sha))
            .map(|(rel, _)| rel.to_string())
            .collect();
        if mismatches.is_empty() {
            return Ok(GateReport { build: Some(build.name.to_string()), mismatches });
        }
        if closest.as_ref().is_none_or(|c| mismatches.len() < c.len()) {
            closest = Some(mismatches);
        }
    }
    Ok(GateReport { build: None, mismatches: closest.unwrap_or_default() })
}

#[cfg(test)]
pub(crate) fn fake_build(root: &Path, files: &[&str]) -> &'static [KnownBuild] {
    // Leaked on purpose: tests need 'static tables like KNOWN_BUILDS.
    let pairs: Vec<(&'static str, &'static str)> = files
        .iter()
        .map(|f| {
            let rel: &'static str = Box::leak(f.to_string().into_boxed_str());
            let sha: &'static str = Box::leak(sha256_file(&root.join(f)).unwrap().into_boxed_str());
            (rel, sha)
        })
        .collect();
    Box::leak(vec![KnownBuild { name: "test build", files: Box::leak(pairs.into_boxed_slice()) }].into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{put, spaced_tempdir};

    #[test]
    fn many_files_hash_in_the_order_given_and_a_missing_one_is_an_error_in_place() {
        let g = spaced_tempdir();
        let mut paths = Vec::new();
        for i in 0..40 {
            put(g.path(), &format!("f{i}.bin"), format!("file {i}").as_bytes());
            paths.push(g.path().join(format!("f{i}.bin")));
        }
        paths.insert(7, g.path().join("missing.bin"));
        let all = sha256_files(&paths);
        assert_eq!(all.len(), 41);
        for (p, got) in paths.iter().zip(&all) {
            match sha256_file(p) {
                Ok(want) => assert_eq!(got.as_ref().unwrap(), &want),
                Err(_) => assert!(got.is_err(), "{p:?}"),
            }
        }
        assert!(all[7].is_err());
        assert!(sha256_files(&[]).is_empty());
    }

    #[test]
    fn gate_accepts_matching_files_and_names_mismatches() {
        let g = spaced_tempdir();
        put(g.path(), "a.bin", b"a");
        put(g.path(), "b.bin", b"b");
        let known = fake_build(g.path(), &["a.bin", "b.bin"]);
        assert!(check_build(g.path(), known).unwrap().supported());
        put(g.path(), "b.bin", b"x");
        let r = check_build(g.path(), known).unwrap();
        assert!(!r.supported());
        assert_eq!(r.mismatches, vec!["b.bin".to_string()]);
    }

    #[test]
    #[ignore]
    fn real_game_passes_gate() {
        let game = std::env::var("OBSIDIAN_GAME_DIR").expect("OBSIDIAN_GAME_DIR");
        assert_eq!(check_build(Path::new(&game), KNOWN_BUILDS).unwrap().build.as_deref(), Some("Steam 6001293"));
    }
}
