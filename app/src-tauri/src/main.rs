// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    // `Obsidian launch %command%`, a Steam launch option, runs the game with no window. It goes
    // before the app starts: an open window would otherwise take the call and the game never start.
    if std::env::args_os().nth(1).is_some_and(|a| a == "launch") {
        return obsidian_core::cli::main();
    }
    obsidian_app_lib::run();
    ExitCode::SUCCESS
}
