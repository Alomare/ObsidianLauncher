//! End-to-end checks of the `obsidian` binary on throwaway folders.

use assert_cmd::Command;
use obsidian_core::bundle::{self, Bundle, Entry, Variant};

fn spaced() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("obsidian cli (x86) ").tempdir().unwrap()
}

fn obsidian(game: &std::path::Path, data: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("obsidian").unwrap();
    c.arg("--game").arg(game).arg("--data-dir").arg(data);
    // Never the player's real saves, and Linux wants no Proton prefix for a throwaway game.
    c.env("OBSIDIAN_APPDATA", data.join("appdata"));
    c
}

fn bundle_with(data: &[u8]) -> Bundle {
    let v = Variant { lang: 0, data: data.to_vec(), stream_size: 0 };
    Bundle { props: [9; 256], entries: vec![Entry { type_hash: 1, name_hash: 1, flags: 0, variants: vec![v] }] }
}

#[test]
fn help_lists_every_command() {
    let out = Command::cargo_bin("obsidian").unwrap().arg("--help").assert().success().get_output().stdout.clone();
    let text = String::from_utf8(out).unwrap();
    for cmd in ["game", "import", "list", "order", "enable", "disable", "conflicts", "deploy", "purge", "status", "updates", "update", "restore-starter"] {
        assert!(text.contains(cmd), "missing {cmd}");
    }
}

#[test]
fn unknown_build_is_reported_and_doesnt_block_deploy() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("binaries/x64")).unwrap();
    std::fs::write(game.join("binaries/x64/helldivers.exe"), b"not the real game").unwrap();
    let data = t.path().join("data");
    let out = obsidian(&game, &data).arg("status").assert().success().get_output().stdout.clone();
    assert!(String::from_utf8(out).unwrap().contains("Game build: not recognised"));
    let err = obsidian(&game, &data).arg("deploy").assert().get_output().stderr.clone();
    assert!(!String::from_utf8(err).unwrap().contains("not recognised"), "the build check no longer stops a deploy");
}

#[test]
fn purge_works_with_a_broken_mod_in_the_library() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let data = t.path().join("data");
    std::fs::create_dir_all(data.join("mods/broken")).unwrap();
    std::fs::write(data.join("mods/broken/obsidian.json"), b"{ not json").unwrap();
    obsidian(&game, &data).arg("purge").assert().success();
}

#[test]
fn a_legacy_mod_needs_the_original_game_files() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    bundle::write_file(&game.join("contents/00000000000000aa"), &bundle_with(b"old")).unwrap();
    let m = t.path().join("My Mod (v2)");
    std::fs::create_dir_all(&m).unwrap();
    bundle::write_file(&m.join("00000000000000aa"), &bundle_with(b"new")).unwrap();
    let data = t.path().join("data");
    let out = obsidian(&game, &data).arg("import").arg(&m).assert().failure().get_output().stderr.clone();
    assert!(String::from_utf8(out).unwrap().contains("verify the game files in Steam"));
    let out = obsidian(&game, &data).arg("list").assert().success().get_output().stdout.clone();
    assert!(!String::from_utf8(out).unwrap().contains("legacy.my-mod-v2"));
}

fn settings_path(appdata: &std::path::Path) -> std::path::PathBuf {
    appdata.join("Arrowhead/Helldivers/ObsidianLauncher/settings.txt")
}

#[test]
fn block_public_and_set_write_settings_for_a_starter_mod() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let (data, appdata) = (t.path().join("data"), t.path().join("appdata"));
    let run = |args: &[&str]| {
        let mut c = obsidian(&game, &data);
        c.env("OBSIDIAN_APPDATA", &appdata).args(args);
        c
    };
    let list = String::from_utf8(run(&["list"]).assert().success().get_output().stdout.clone()).unwrap();
    assert!(list.contains("[on ] obsidian.qol") && list.contains("[off] obsidian.trainer"), "only Obsidian Patches starts on: {list}");
    run(&["block-public", "on"]).assert().success();
    run(&["set", "obsidian.qol", "skip_intro", "false"]).assert().success();
    let s = std::fs::read_to_string(settings_path(&appdata)).unwrap();
    assert!(s.contains("consent\t0\n") && s.contains("option\tobsidian.qol\tskip_intro\ttoggle\t0\n"), "{s}");
    run(&["set", "obsidian.qol", "skip_intro", "7"]).assert().failure();
}

#[test]
fn profile_commands_switch_what_list_shows() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let (data, appdata) = (t.path().join("data"), t.path().join("appdata"));
    let run = |args: &[&str]| {
        let mut c = obsidian(&game, &data);
        c.env("OBSIDIAN_APPDATA", &appdata).args(args);
        c
    };
    let out = |args: &[&str]| String::from_utf8(run(args).assert().success().get_output().stdout.clone()).unwrap();
    run(&["enable", "obsidian.qol"]).assert().success();
    run(&["key", "obsidian.qol", "nope", "f9"]).assert().failure();
    run(&["profile", "new", "Clean"]).assert().success();
    run(&["profile", "new", "Copy", "--copy", "Default"]).assert().success();
    assert!(out(&["profile", "list"]).contains("* Default"), "the active profile is marked");
    run(&["profile", "use", "clean"]).assert().success();
    assert!(out(&["list"]).contains("[off] obsidian.qol"));
    run(&["profile", "use", "Copy"]).assert().success();
    assert!(out(&["list"]).contains("[on ] obsidian.qol"));
    run(&["profile", "use", "Missing"]).assert().failure();
    run(&["profile", "new", "a/b"]).assert().failure();
}

#[test]
fn removed_mod_leaves_every_profile() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let m = t.path().join("My Mod (v2)");
    std::fs::create_dir_all(&m).unwrap();
    std::fs::write(m.join("obsidian.json"), br#"{"obsidian":1,"id":"me.my-mod","version":"1.0.0","name":{"en":"My Mod"}}"#).unwrap();
    let (data, appdata) = (t.path().join("data"), t.path().join("appdata"));
    let run = |args: &[&str]| {
        let mut c = obsidian(&game, &data);
        c.env("OBSIDIAN_APPDATA", &appdata).args(args);
        c
    };
    run(&["import", m.to_str().unwrap()]).assert().success();
    run(&["enable", "me.my-mod"]).assert().success();
    run(&["profile", "new", "Other", "--copy", "Default"]).assert().success();
    run(&["remove", "me.my-mod"]).assert().success();
    for name in ["Default", "Other"] {
        run(&["profile", "use", name]).assert().success();
        let list = String::from_utf8(run(&["list"]).assert().success().get_output().stdout.clone()).unwrap();
        assert!(!list.contains("my-mod"), "{name}: {list}");
        let saved = std::fs::read_to_string(data.join(format!("profiles/{name}.json"))).unwrap();
        assert!(!saved.contains("my-mod"), "{name}: {saved}");
    }
}

#[test]
fn the_starter_pack_stays_deleted_until_it_is_restored() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let (data, appdata) = (t.path().join("data"), t.path().join("appdata"));
    let run = |args: &[&str]| {
        let mut c = obsidian(&game, &data);
        c.env("OBSIDIAN_APPDATA", &appdata).args(args);
        c
    };
    let list = || String::from_utf8(run(&["list"]).assert().success().get_output().stdout.clone()).unwrap();
    assert!(list().contains("1. [on ] obsidian.api"), "first in the load order, on for Obsidian Patches");
    run(&["remove", "obsidian.qol"]).assert().success();
    assert!(!list().contains("obsidian.qol"), "and it stays gone at the next start");
    run(&["restore-starter"]).assert().success();
    let back = list();
    assert!(back.contains("3. [on ] obsidian.qol"), "{back}");
}

#[test]
fn status_names_the_crash_culprit() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("binaries/x64")).unwrap();
    std::fs::write(game.join("binaries/x64/helldivers.exe"), b"not the real game").unwrap();
    let appdata = t.path().join("appdata");
    let logs = appdata.join("Arrowhead/Helldivers/ObsidianLauncher/logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("loader.log"), "10:00:00 loading a.b
10:00:01 started a.b
10:00:01 loading c.d
").unwrap();
    let out = obsidian(&game, &t.path().join("data")).env("OBSIDIAN_APPDATA", &appdata).arg("status").assert().success().get_output().stdout.clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("The last game launch stopped while loading c.d"), "{text}");
}

/// A deploy that wrote nothing, as after `obsidian deploy` with every mod off.
fn empty_deploy(data: &std::path::Path) {
    std::fs::create_dir_all(data).unwrap();
    std::fs::write(data.join("deploy-state.json"), r#"{"build":"Steam 6001293","files":[],"mods":[],"time":"t"}"#).unwrap();
}

#[test]
fn status_says_vanilla_when_the_deploy_wrote_nothing() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let data = t.path().join("data");
    empty_deploy(&data);
    let out = obsidian(&game, &data).env("OBSIDIAN_APPDATA", t.path().join("appdata")).arg("status").assert().success().get_output().stdout.clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Deployed: nothing, the game is vanilla"), "{text}");
}

#[test]
fn a_guardrail_asks_for_a_deploy_when_the_loader_is_not_in_the_game() {
    let t = spaced();
    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("contents")).unwrap();
    let data = t.path().join("data");
    empty_deploy(&data);
    let out = obsidian(&game, &data).env("OBSIDIAN_APPDATA", t.path().join("appdata")).args(["block-achievements", "on"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Run obsidian deploy to apply."), "{text}");
}
