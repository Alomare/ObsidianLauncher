//! Checks a repository before Obsidian sends it to the Store (`github::open_submission`). The Store
//! index runs the same checks on the pull request (`tools/validate.py` there), since a pull request
//! can also come from elsewhere; keep the two in step.
//!
//! A submission must be the player's own public repository, with a newest stable release holding one
//! mod ZIP whose manifest Obsidian accepts, a declared scope, and readable text: a name, a summary and
//! a README written in words, not keyboard mashing or placeholders.

use std::path::Path;

use serde::Serialize;

use crate::github::{self, Client};
use crate::manifest::Scope;
use crate::updates::{GitHub, github_repo, stage_latest};
use crate::{ObsidianError, Result};

/// One thing that keeps the repository out, by code for the window's catalog
/// (`app.submit.problem.<code>`), with the detail when there is one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Problem {
    pub code: &'static str,
    pub detail: String,
}

fn problem(code: &'static str, detail: impl Into<String>) -> Problem {
    Problem { code, detail: detail.into() }
}

/// What the Store would show, for the player to look over before sending.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Preview {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub scope: String,
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Checked {
    pub repo: String,
    pub problems: Vec<Problem>,
    pub preview: Option<Preview>,
}

/// Placeholders and keyboard rows that are never a real name, summary or README.
const JUNK: &[&str] = &["lorem ipsum", "asdf", "qwerty", "zxcv", "hjkl", "test test", "todo", "tbd", "placeholder", "description here", "your description", "mod name", "my mod", "example mod", "untitled"];

/// Why `text` isn't readable as a `kind` ("name", "summary", "readme"), or None when it is.
/// The same rules as `text_problem` in the Store index's tools/validate.py.
pub fn text_problem(text: &str, kind: &str) -> Option<&'static str> {
    let text = text.trim();
    let n = text.chars().count();
    let (min, max) = match kind {
        "name" => (3, 40),
        "summary" => (20, 500),
        _ => (150, usize::MAX),
    };
    if n < min {
        return Some("short");
    }
    if n > max {
        return Some("long");
    }
    let lower = text.to_lowercase();
    let squashed: String = lower.chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect();
    if JUNK.iter().any(|j| squashed.trim() == *j || (kind != "name" && squashed.contains(j))) || squashed.trim() == "test" || squashed.trim() == "mod" {
        return Some("placeholder");
    }
    let visible: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let letters = visible.iter().filter(|c| c.is_alphabetic()).count();
    if letters * 2 < visible.len() {
        return Some("symbols");
    }
    let mut run = 1;
    let mut last = None;
    for c in lower.chars() {
        run = if Some(c) == last { run + 1 } else { 1 };
        last = Some(c);
        if run >= 5 {
            return Some("repeated");
        }
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let ascii: Vec<char> = lower.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    // Scripts written without spaces (Chinese, Japanese) have no words to count.
    let latin = ascii.len() * 2 >= letters;
    if kind != "name" && latin {
        let need = if kind == "summary" { 4 } else { 30 };
        if words.len() < need {
            return Some("words");
        }
    }
    if words.iter().any(|w| w.chars().count() > 30 && !w.contains("://") && !w.contains('/') && !w.contains('`')) {
        return Some("gibberish");
    }
    // Latin script only: a fair share of vowels, as in any language written with it.
    if ascii.len() >= 10 && latin {
        let vowels = ascii.iter().filter(|c| "aeiouy".contains(**c)).count() as f64 / ascii.len() as f64;
        if !(0.2..=0.7).contains(&vowels) {
            return Some("gibberish");
        }
    }
    let upper = text.chars().filter(|c| c.is_uppercase()).count();
    if kind != "name" && letters >= 12 && upper * 10 >= letters * 8 {
        return Some("shouting");
    }
    None
}

/// README text without the Markdown that isn't words: code blocks, pictures, links' addresses, tags.
fn readme_words(md: &str) -> String {
    let mut out = String::new();
    let mut fence = false;
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            fence = !fence;
            continue;
        }
        if fence {
            continue;
        }
        let mut l = String::new();
        let mut chars = line.chars().peekable();
        let mut depth = 0;
        while let Some(c) = chars.next() {
            match c {
                '<' => depth += 1,
                '>' if depth > 0 => depth -= 1,
                ']' if chars.peek() == Some(&'(') => {
                    // Skip the address of a link or picture.
                    for d in chars.by_ref() {
                        if d == ')' {
                            break;
                        }
                    }
                }
                _ if depth == 0 && !"#*_`|![".contains(c) => l.push(c),
                _ => {}
            }
        }
        out.push_str(l.trim());
        out.push('\n');
    }
    out
}

fn en(map: &std::collections::BTreeMap<String, String>) -> String {
    map.get("en").or_else(|| map.values().next()).cloned().unwrap_or_default()
}

/// Runs every check on `repo` as the signed-in player. `store` is the Store as read now.
pub fn check(data: &Path, c: &Client, releases: &GitHub, repo: &str) -> Result<Checked> {
    if !c.signed_in() {
        return Err(ObsidianError::GitHubSignIn);
    }
    let mut out = Checked::default();
    let Some(name) = github::repo_name(repo) else {
        out.problems.push(problem("repoName", repo.trim()));
        return Ok(out);
    };
    out.repo = name.clone();
    let Some(info) = github::repo(c, &name)? else {
        out.problems.push(problem("repoMissing", &name));
        return Ok(out);
    };
    out.repo = info.full_name.clone();
    if info.private {
        out.problems.push(problem("repoPrivate", ""));
    }
    if info.fork {
        out.problems.push(problem("repoFork", ""));
    }
    if info.archived {
        out.problems.push(problem("repoArchived", ""));
    }
    if !info.push {
        out.problems.push(problem("notAuthor", &info.owner));
    }
    let store = crate::store::fetch(data)?;
    let lower = out.repo.to_lowercase();
    if store.mods.iter().any(|m| m.repo.to_lowercase() == lower) {
        out.problems.push(problem("listed", ""));
        return Ok(out);
    }
    let Some(r) = github_repo(&format!("https://github.com/{}", out.repo)) else {
        out.problems.push(problem("repoName", &out.repo));
        return Ok(out);
    };
    let (staged, version) = match stage_latest(releases, &r) {
        Ok(s) => s,
        Err(ObsidianError::NoRelease(_)) => {
            out.problems.push(problem("noRelease", ""));
            return Ok(out);
        }
        Err(ObsidianError::NoUpdateAsset(_)) => {
            out.problems.push(problem("noZip", ""));
            return Ok(out);
        }
        Err(ObsidianError::BadManifest { field, reason }) => {
            out.problems.push(problem("badManifest", format!("{field}: {reason}")));
            return Ok(out);
        }
        Err(e) => return Err(e),
    };
    let m = &staged.manifest;
    if m.version != version {
        out.problems.push(problem("versionMismatch", format!("{} / {}", m.version, version)));
    }
    if !m.version.pre.is_empty() {
        out.problems.push(problem("prerelease", m.version.to_string()));
    }
    if m.scope == Scope::Unspecified {
        out.problems.push(problem("noScope", ""));
    }
    if store.mods.iter().any(|x| x.id == m.id) {
        out.problems.push(problem("idTaken", &m.id));
    }
    if m.id.starts_with("obsidian.") && !info.owner.eq_ignore_ascii_case("Alomare") {
        out.problems.push(problem("idReserved", &m.id));
    }
    for need in &m.requires {
        if !store.mods.iter().any(|x| &x.id == need) {
            out.problems.push(problem("requiresMissing", need));
        }
    }
    let display = en(&m.name);
    let summary = m.description.as_ref().map(en).unwrap_or_default();
    if let Some(why) = text_problem(&display, "name") {
        out.problems.push(problem("nameText", why));
    }
    if let Some(why) = text_problem(&summary, "summary") {
        out.problems.push(problem("summaryText", why));
    }
    let readme = c.call_raw(&format!("/repos/{}/readme", out.repo))?;
    match readme {
        None => out.problems.push(problem("readmeMissing", "")),
        Some(text) => {
            if let Some(why) = text_problem(&readme_words(&text), "readme") {
                out.problems.push(problem("readmeText", why));
            }
        }
    }
    let scope = serde_json::to_value(m.scope).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    out.preview = Some(Preview { id: m.id.clone(), name: display, summary, version: m.version.to_string(), scope, authors: m.authors.clone() });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_text_passes() {
        assert_eq!(text_problem("Better HUD", "name"), None);
        assert_eq!(text_problem("Photo Mode", "name"), None);
        assert_eq!(text_problem("Shows ammo, grenades and stratagem cooldowns in a cleaner layout.", "summary"), None);
        assert_eq!(text_problem("Mostra munição e granadas num layout mais limpo para o jogador.", "summary"), None);
        assert_eq!(text_problem("在游戏中显示弹药和手雷，界面更清晰，适合所有玩家使用的模组", "summary"), None, "no vowel rule outside Latin script");
        let readme = "This mod replaces the heads-up display of the game with a cleaner one. ".repeat(4);
        assert_eq!(text_problem(&readme, "readme"), None);
    }

    #[test]
    fn gibberish_and_placeholders_fail() {
        assert_eq!(text_problem("ab", "name"), Some("short"));
        assert_eq!(text_problem("asdf", "name"), Some("placeholder"));
        assert_eq!(text_problem("Test", "name"), Some("placeholder"));
        assert_eq!(text_problem("My Mod", "name"), Some("placeholder"));
        assert_eq!(text_problem("Lorem ipsum dolor sit amet consectetur", "summary"), Some("placeholder"));
        assert_eq!(text_problem("sdfkjh qwrtpl zxcvbn mnbvcx lkjhg", "summary"), Some("placeholder"));
        assert_eq!(text_problem("sdfkjh wrtpl bvcxzm nbvcx lkjhgf", "summary"), Some("gibberish"));
        assert_eq!(text_problem("this mod is goooooood for you all", "summary"), Some("repeated"));
        assert_eq!(text_problem("!!! ### $$$ %%% ^^^ &&& ***", "summary"), Some("symbols"));
        assert_eq!(text_problem("THIS MOD IS THE BEST MOD EVER MADE", "summary"), Some("shouting"));
        assert_eq!(text_problem("Wonderful, essential modification", "summary"), Some("words"));
        assert_eq!(text_problem("A mod abcdefghijklmnopqrstuvwxyzabcdefghij that does things", "summary"), Some("gibberish"));
        assert_eq!(text_problem("Short readme.", "readme"), Some("short"));
    }

    #[test]
    fn readme_words_drop_markdown_that_is_not_text() {
        let md = "# Title\n\n![shot](img/a.png)\n\nSee [the wiki](https://x.y/z) for <b>more</b>.\n\n```lua\nlocal x = 1\n```\n";
        let w = readme_words(md);
        assert!(w.contains("Title") && w.contains("See the wiki for more."));
        assert!(!w.contains("https") && !w.contains("local x") && !w.contains("img/a.png"));
    }
}
