//! Times the reads the app makes on every click and at start, against a real library and game:
//! `cargo run --release -p obsidian-core --example profile_reads [data dir]`.

use std::time::Instant;

use obsidian_core::compose::plan_for;
use obsidian_core::deploy::{changes_waiting, load_record, status};
use obsidian_core::game::find_game;
use obsidian_core::gate::{KNOWN_BUILDS, check_build, check_build_cached};
use obsidian_core::library::{Library, Profile};

fn time<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    println!("{name:<40} {:>9.1} ms", start.elapsed().as_secs_f64() * 1000.0);
    out
}

fn main() {
    let data = obsidian_core::paths::data_dir(std::env::args().nth(1).as_deref().map(std::path::Path::new));
    let game = time("find game", || find_game(None)).expect("the game");
    let contents = game.contents();
    let lib = time("library open", || Library::open(&data)).unwrap();
    time("library list_and_broken", || lib.list_and_broken()).unwrap();
    let mut profile = time("profile load", || Profile::load(&data)).unwrap();
    time("profile sync", || profile.sync(&lib)).unwrap();
    for round in ["first", "second"] {
        time(&format!("plan ({round})"), || plan_for(&lib, &profile, Some(&contents))).unwrap();
    }
    time("changes waiting", || changes_waiting(&lib, &profile, &data, Some(&contents))).unwrap();
    time("changes waiting (second)", || changes_waiting(&lib, &profile, &data, Some(&contents))).unwrap();
    let comp = plan_for(&lib, &profile, Some(&contents)).unwrap();
    let dir = time("find loader dir", || obsidian_core::loader::find_loader_dir()).unwrap();
    time("settings_for", || obsidian_core::loader::settings_for(&lib, &profile)).unwrap();
    time("fingerprint", || obsidian_core::deploy::fingerprint(&comp, &profile, &dir)).unwrap();
    let _ = time("load record", || load_record(&data));
    time("build check, kept (first)", || check_build_cached(&game.root, KNOWN_BUILDS)).unwrap();
    time("build check, kept (second)", || check_build_cached(&game.root, KNOWN_BUILDS)).unwrap();
    time("build check, uncached", || check_build(&game.root, KNOWN_BUILDS)).unwrap();
    time("status", || status(&game, &data, KNOWN_BUILDS)).unwrap();
}
