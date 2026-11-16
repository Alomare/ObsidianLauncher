//! Fuzzes the parsers that read files Obsidian does not control, and drives deploy and purge on a fake
//! game many times: cycles, deploys killed midway, deploys racing each other. Seeds are fixed, so a
//! failure repeats. `OBSIDIAN_FUZZ=<n>` sets the fuzz rounds (default 600; the 2.0 review ran 3000).
//! The 120 mod timing run is ignored by default:
//!   cargo test -p obsidian-core --test robustness --release -- --ignored --nocapture

use obsidian_core::bundle::{self, Bundle, Entry, Variant};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// xorshift64*: deterministic, so a failure repeats with the same seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    fn text(&mut self, n: usize, alphabet: &[u8]) -> String {
        (0..n).map(|_| alphabet[self.below(alphabet.len())] as char).collect()
    }
}

fn iter() -> usize {
    std::env::var("OBSIDIAN_FUZZ").ok().and_then(|v| v.parse().ok()).unwrap_or(600)
}

fn catch<F: FnOnce() + std::panic::UnwindSafe>(what: &str, input: &dyn std::fmt::Debug, f: F) {
    if std::panic::catch_unwind(f).is_err() {
        panic!("{what} panicked on {input:?}");
    }
}

#[test]
fn fuzz_settings_parse_never_panics_and_render_is_stable() {
    let mut r = Rng(0x5eed_0001);
    let words: [&str; 14] = ["consent", "key", "mode", "pad", "option", "war", "sandbox", "1", "0", "a.b", "toggle", "f8", "\t", "\r"];
    let t = Instant::now();
    let n = iter();
    for _ in 0..n {
        let mut text = String::new();
        for _ in 0..r.below(20) {
            for _ in 0..r.below(6) {
                text.push_str(words[r.below(words.len())]);
                if r.below(2) == 0 { text.push('\t'); }
            }
            if r.below(10) == 0 { text.push_str(&String::from_utf8_lossy(&r.bytes(8))); }
            text.push('\n');
        }
        let input = text.clone();
        catch("settings::parse", &input, move || {
            let s = obsidian_core::settings::parse(&text);
            let once = obsidian_core::settings::render(&s);
            let twice = obsidian_core::settings::render(&obsidian_core::settings::parse(&once));
            assert_eq!(once, twice, "render(parse(render)) moved");
        });
    }
    eprintln!("settings fuzz: {n} files in {:?}", t.elapsed());
}

fn small_bundle(r: &mut Rng, entries: usize) -> Bundle {
    let entries = (0..entries)
        .map(|i| Entry {
            type_hash: 0x10 + (i as u64 % 3),
            name_hash: r.next() | 1,
            flags: 0,
            variants: vec![Variant { lang: 0, data: { let n = 16 + r.below(512); r.bytes(n) }, stream_size: 0 }],
        })
        .collect();
    let mut b = Bundle { props: [0; 256], entries };
    b.sort_entries();
    b
}

#[test]
fn fuzz_bundle_readers_never_panic_on_corrupt_files() {
    let mut r = Rng(0x5eed_0002);
    let dir = tempfile::tempdir().unwrap();
    let t = Instant::now();
    let rounds = iter() / 3;
    for i in 0..rounds {
        let n = 1 + r.below(6);
        let b = small_bundle(&mut r, n);
        let mut file = bundle::compress(&bundle::serialize_payload(&b));
        // Corrupt it: flip bytes, cut it short, or pad it with junk.
        match i % 4 {
            0 => { for _ in 0..1 + r.below(8) { let at = r.below(file.len()); file[at] ^= 1 << r.below(8); } }
            1 => { let keep = r.below(file.len()); file.truncate(keep); }
            2 => { let junk = { let n = r.below(64); r.bytes(n) }; file.extend(junk); }
            _ => { file = { let n = r.below(300); r.bytes(n) }; }
        }
        let p = dir.path().join(format!("b{}", i % 7));
        std::fs::write(&p, &file).unwrap();
        let (f2, p2) = (file.clone(), p.clone());
        catch("bundle readers", &(i, file.len()), move || {
            let _ = bundle::decompress(&f2).and_then(|pl| bundle::parse_payload(&pl));
            let _ = bundle::read_file(&p2);
            let _ = bundle::read_props(&p2);
            let _ = bundle::read_index(&p2);
            if let Ok(mut rd) = bundle::Reader::open(&p2) { let _ = rd.heads(); }
        });
    }
    eprintln!("bundle fuzz: {rounds} corrupt files in {:?}", t.elapsed());
}

#[test]
fn fuzz_small_parsers_never_panic() {
    let mut r = Rng(0x5eed_0003);
    let any = b"abcxyz019_+.-/: \"{}\\\t\nAZ\xc3\xa9";
    for _ in 0..iter() {
        let s = { let n = r.below(80); r.text(n, any) };
        let s2 = s.clone();
        catch("small parsers", &s, move || {
            let _ = obsidian_core::options::is_pad_combo(&s2);
            let _ = obsidian_core::options::is_key_name(&s2);
            let _ = obsidian_core::vdf::library_paths(&s2);
            let _ = obsidian_core::updates::github_repo(&s2);
        });
    }
    // A libraryfolders.vdf shaped file with junk in the right places.
    for _ in 0..iter() / 3 {
        let body = format!("\"libraryfolders\"\n{{\n\"0\"\n{{\n\"path\" \"{}\"\n\"apps\" {{ \"{}\" \"1\" }}\n", r.text(20, any), r.text(6, b"0123456789"));
        let cut = r.below(body.len() + 1);
        let s = body[..body.char_indices().map(|(i, _)| i).filter(|i| *i <= cut).last().unwrap_or(0)].to_string();
        catch("vdf", &s.clone(), move || { let _ = obsidian_core::vdf::library_paths(&s); });
    }
}

#[test]
fn fuzz_manifest_loading_never_panics() {
    let mut r = Rng(0x5eed_0004);
    let dir = tempfile::tempdir().unwrap();
    let fields = ["\"obsidian\":1", "\"id\":\"a.b\"", "\"version\":\"1.0.0\"", "\"name\":{\"en\":\"X\"}", "\"scope\":\"session\"", "\"entry\":\"main.lua\"",
        "\"options\":[{\"id\":\"o\",\"type\":\"number\",\"default\":5,\"min\":1,\"max\":3}]", "\"keybinds\":[{\"id\":\"k\",\"default\":\"f8\",\"pad\":\"o\"}]",
        "\"requires\":[\"a.b\"]", "\"secondary_scopes\":[{\"id\":\"p\",\"scope\":\"client\"}]", "\"load\":{\"after\":[1]}", "\"version\":\"x\"", "\"options\":7",
        "\"option_groups\":[{}]", "\"resources\":[{\"name\":null}]", "\"min_obsidian\":\"99.0.0\""];
    let t = Instant::now();
    let rounds = iter() / 3;
    for i in 0..rounds {
        let picked: Vec<&str> = (0..r.below(9)).map(|_| fields[r.below(fields.len())]).collect();
        let mut json = format!("{{{}}}", picked.join(","));
        if i % 5 == 0 { let cut = r.below(json.len()); json.truncate(cut); }
        let m = dir.path().join(format!("m{}", i % 5));
        std::fs::create_dir_all(&m).unwrap();
        std::fs::write(m.join("obsidian.json"), &json).unwrap();
        obsidian_core::manifest::forget(&m);
        let m2 = m.clone();
        catch("manifest", &json, move || {
            if let Ok(man) = obsidian_core::manifest::load_manifest(&m2) {
                let _ = man.validate(&m2);
                let _ = man.option_specs();
                let _ = man.save_editor();
                let _ = man.load_rules();
            }
        });
    }
    eprintln!("manifest fuzz: {rounds} files in {:?}", t.elapsed());
}

// ---- deploy and purge on a fake game -------------------------------------------------------------

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_obsidian-cli"))
}

struct World {
    _t: tempfile::TempDir,
    game: PathBuf,
    data: PathBuf,
    appdata: PathBuf,
}

impl World {
    fn new() -> World {
        // A space and brackets in the path, as under Program Files (x86).
        let t = tempfile::Builder::new().prefix("obsidian robustness (x86) ").tempdir().unwrap();
        let game = t.path().join("game");
        std::fs::create_dir_all(game.join("contents")).unwrap();
        let (data, appdata) = (t.path().join("data"), t.path().join("appdata"));
        World { game, data, appdata, _t: t }
    }
    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(exe());
        c.arg("--game").arg(&self.game).arg("--data-dir").arg(&self.data).env("OBSIDIAN_APPDATA", &self.appdata).args(args);
        c
    }
    fn ok(&self, args: &[&str]) -> (String, Duration) {
        let t = Instant::now();
        let out = self.cmd(args).output().unwrap();
        let took = t.elapsed();
        assert!(out.status.success(), "{args:?} failed: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        (String::from_utf8_lossy(&out.stdout).into_owned(), took)
    }
    fn contents(&self) -> BTreeMap<String, Vec<u8>> {
        let mut m = BTreeMap::new();
        for e in std::fs::read_dir(self.game.join("contents")).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_file() {
                m.insert(e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap());
            }
        }
        m
    }
}

fn bundle_name(i: usize) -> String {
    format!("{:016x}", 0xa0 + i)
}

/// A fake game of `bundles` bundles, and `mods` mods, each changing entries of two or three of them
/// (so mods overlap). Returns the vanilla contents.
fn fake_game(w: &World, r: &mut Rng, bundles: usize, mods: usize) -> BTreeMap<String, Vec<u8>> {
    let texture = obsidian_core::hash::hash_name("texture");
    let mut vanilla = Vec::new();
    for i in 0..bundles {
        let mut b = small_bundle(r, 8);
        for e in &mut b.entries { e.type_hash = texture; }
        b.sort_entries();
        bundle::write_file(&w.game.join("contents").join(bundle_name(i)), &b).unwrap();
        vanilla.push(b);
    }
    // The boot bundle the loader goes into, with the boot script in it.
    let lua = obsidian_core::hash::hash_name("lua");
    let boot = Bundle { props: [0; 256], entries: vec![Entry { type_hash: lua, name_hash: obsidian_core::hash::hash_name(obsidian_core::loader::ENTRY), flags: 0,
        variants: vec![Variant { lang: 0, data: b"-- boot".to_vec(), stream_size: 0 }] }] };
    bundle::write_file(&w.game.join("contents").join(obsidian_core::loader::BUNDLE), &boot).unwrap();
    let before = w.contents();
    // The Kit's Patches change real game bundles this fake game lacks.
    w.ok(&["disable", "obsidian.qol"]);
    let src = w.game.parent().unwrap().join("mods-src");
    for m in 0..mods {
        let dir = src.join(format!("Review Mod {m}"));
        std::fs::create_dir_all(dir.join("res")).unwrap();
        let mut res = Vec::new();
        for k in 0..2 + r.below(3) {
            let i = r.below(bundles);
            let e = r.below(vanilla[i].entries.len());
            let data = { let n = 64 + r.below(256); r.bytes(n) };
            std::fs::write(dir.join(format!("res/r{k}.texture")), data).unwrap();
            res.push(format!(r##"{{"type":"texture","name":"#{:016x}","bundle":"{}","data":"res/r{k}.texture"}}"##, vanilla[i].entries[e].name_hash, bundle_name(i)));
        }
        let manifest = format!(r#"{{"obsidian":1,"id":"review.mod-{m}","version":"1.0.0","name":{{"en":"Review Mod {m}"}},"scope":"client","resources":[{}]}}"#, res.join(","));
        std::fs::write(dir.join("obsidian.json"), manifest).unwrap();
        w.ok(&["import", dir.to_str().unwrap()]);
    }
    before
}

fn ids(w: &World) -> Vec<String> {
    let (list, _) = w.ok(&["list"]);
    list.lines().filter_map(|l| l.split(']').nth(1)?.split_whitespace().next()).filter(|id| id.starts_with("review.")).map(str::to_string).collect()
}

#[test]
fn deploy_and_purge_cycles_leave_the_game_vanilla() {
    let w = World::new();
    let mut r = Rng(0x5eed_0005);
    let t = Instant::now();
    let vanilla = fake_game(&w, &mut r, 24, 30);
    eprintln!("import of 30 mods: {:?}", t.elapsed());
    let all = ids(&w);
    assert_eq!(all.len(), 30, "{all:?}");
    let mut times = Vec::new();
    for round in 0..4 {
        // A different set each round.
        for id in &all {
            let on = r.below(3) != 0;
            w.ok(&[if on { "enable" } else { "disable" }, id]);
        }
        let (_, d) = w.ok(&["deploy"]);
        let modded = w.contents();
        assert_ne!(modded, vanilla, "round {round}: the deploy changed nothing");
        let (_, d2) = w.ok(&["deploy"]);
        assert_eq!(w.contents(), modded, "round {round}: a second deploy of the same set changed the game");
        let (_, p) = w.ok(&["purge"]);
        assert_eq!(w.contents(), vanilla, "round {round}: purge did not restore the game");
        times.push((d, d2, p));
    }
    for (i, (d, d2, p)) in times.iter().enumerate() {
        eprintln!("round {i}: deploy {d:?}, same deploy again {d2:?}, purge {p:?}");
    }
}

#[test]
fn a_deploy_killed_midway_is_recovered_by_the_next_purge() {
    let w = World::new();
    let mut r = Rng(0x5eed_0006);
    let vanilla = fake_game(&w, &mut r, 40, 20);
    for id in ids(&w) { w.ok(&["enable", &id]); }
    let mut killed = 0;
    for delay_ms in [5u64, 15, 30, 60, 120, 250] {
        let mut child = w.cmd(&["deploy"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
        std::thread::sleep(Duration::from_millis(delay_ms));
        if child.try_wait().unwrap().is_none() { child.kill().ok(); killed += 1; }
        child.wait().ok();
        // Whatever state that left, purge must bring back the vanilla game, and a deploy must work again.
        let out = w.cmd(&["purge"]).output().unwrap();
        assert!(out.status.success(), "purge after a deploy killed at {delay_ms} ms: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(w.contents(), vanilla, "game not vanilla after a deploy killed at {delay_ms} ms");
        w.ok(&["deploy"]);
        w.ok(&["purge"]);
        assert_eq!(w.contents(), vanilla);
    }
    eprintln!("killed {killed} of 6 deploys midway; every purge restored the game");
}

/// The timing run of the 2.0 review: every command a click runs, with 120 mods.
#[test]
#[ignore = "timing; run with --release -- --ignored --nocapture"]
fn a_big_library_lists_and_deploys_in_time() {
    let w = World::new();
    let mut r = Rng(0x5eed_0007);
    let t = Instant::now();
    fake_game(&w, &mut r, 60, 120);
    eprintln!("import of 120 mods: {:?}", t.elapsed());
    for id in ids(&w) { w.ok(&["enable", &id]); }
    let (_, list) = w.ok(&["list"]);
    // The first read of 120 new mods is the antivirus scanning each file once (about a second on
    // Windows); the second status is what every click costs.
    let (_, first_status) = w.ok(&["status"]);
    let (_, status) = w.ok(&["status"]);
    eprintln!("first status after the import {first_status:?}");
    assert!(status < Duration::from_millis(500), "status took {status:?}");
    let (_, conflicts) = w.ok(&["conflicts"]);
    let (_, deploy) = w.ok(&["deploy"]);
    let (_, purge) = w.ok(&["purge"]);
    eprintln!("120 mods: list {list:?}, status {status:?}, conflicts {conflicts:?}, deploy {deploy:?}, purge {purge:?}");
    assert!(list < Duration::from_secs(3), "list took {list:?}");
}

#[test]
fn two_deploys_at_once_never_leave_a_mixed_game() {
    let w = World::new();
    let mut r = Rng(0x5eed_0008);
    let vanilla = fake_game(&w, &mut r, 30, 16);
    for id in ids(&w) { w.ok(&["enable", &id]); }
    for _ in 0..4 {
        let a = w.cmd(&["deploy"]).output();
        let kids: Vec<_> = (0..3).map(|_| w.cmd(&["deploy"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped()).spawn().unwrap()).collect();
        let a = a.unwrap();
        assert!(a.status.success());
        for k in kids { let _ = k.wait_with_output(); }
        let after_race = w.contents();
        w.ok(&["deploy"]);
        assert_eq!(w.contents(), after_race, "racing deploys left a game a clean deploy changes");
        w.ok(&["purge"]);
        assert_eq!(w.contents(), vanilla);
    }
}
