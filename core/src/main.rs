//! `obsidian`: the command line over obsidian-core.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use obsidian_core::deploy::{ChangesWaiting, DeployOptions, changes_waiting, deploy, purge, status};
use obsidian_core::game::{GameInstall, find_game};
use obsidian_core::gate::KNOWN_BUILDS;
use obsidian_core::hash::{format_name, format_type};
use obsidian_core::import::import_path;
use obsidian_core::library::{InstallOutcome, Library, Profile};
use obsidian_core::manifest::Scope;
use obsidian_core::paths::data_dir;
use obsidian_core::{profiles, starter};
use obsidian_core::updates::{self, GitHub};
use obsidian_core::{ObsidianError, Result};

#[derive(Parser)]
#[command(name = "obsidian", version, about = "Mod manager for HELLDIVERS (2015)")]
struct Cli {
    /// The Helldivers game folder (skips the Steam search)
    #[arg(long, global = true)]
    game: Option<PathBuf>,
    /// Where Obsidian keeps mods, the profile and backups
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the game folder, save folder and build check
    Game,
    /// Import a mod folder or .zip into the library
    Import { path: PathBuf },
    /// List the mods in the library, in load order
    List,
    /// Put these mods first in the load order (later mods win conflicts)
    Order { ids: Vec<String> },
    /// Turn a mod on
    Enable { id: String },
    /// Turn a mod off
    Disable { id: String },
    /// Show which mods change the same resources, without writing anything
    Conflicts,
    /// Write the enabled mods into the game
    Deploy,
    /// Remove everything the last deploy wrote
    Purge,
    /// Show the build check, what is deployed and patches from other tools
    Status,
    /// Allow session mods from other Obsidian users (on or off)
    Consent { state: OnOff },
    /// Set a mod option, for example: obsidian set author.mod distance 7
    Set { id: String, option: String, value: String },
    /// Delete a mod from the library
    Remove { id: String },
    /// Put the Obsidian Kit back as it came, in the active profile
    RestoreStarter,
    /// Set a mod key, for example: obsidian key author.mod toggle f9
    Key { id: String, keybind: String, key: String },
    /// Ask GitHub for new versions of the mods that name a repository
    Updates,
    /// Install new versions from GitHub: of these mods, or of all that have one
    Update { ids: Vec<String> },
    /// List, switch or create profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
}

#[derive(Subcommand)]
enum ProfileAction {
    /// List the profiles; the active one is marked with *
    List,
    /// Make a profile active
    Use { name: String },
    /// Create a profile, empty or a copy of another
    New {
        name: String,
        #[arg(long)]
        copy: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OnOff {
    On,
    Off,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            if matches!(e, ObsidianError::UnsupportedBuild(_)) { ExitCode::from(2) } else { ExitCode::FAILURE }
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let data = data_dir(cli.data_dir.as_deref());
    let game = || find_game(cli.game.as_deref());
    let lib = Library::open(&data)?;
    // Purge and status must work even when a mod folder or the profile is damaged,
    // so only the commands that use the library read and sync it.
    let uses_library = matches!(
        cli.command,
        Command::Import { .. } | Command::List | Command::Order { .. } | Command::Enable { .. } | Command::Disable { .. } | Command::Conflicts
            | Command::Consent { .. } | Command::Set { .. } | Command::Key { .. } | Command::Profile { .. }
            | Command::Remove { .. } | Command::RestoreStarter | Command::Updates | Command::Update { .. }
    );
    let mut profile = Profile::default();
    if uses_library {
        starter::seed(&data)?;
        profile = Profile::load(&data)?;
        profile.sync(&lib)?;
        profile.save(&data)?;
    }
    match cli.command {
        Command::Game => {
            let g = game()?;
            println!("Game folder: {}", g.root.display());
            println!("Save folder: {}", g.saves_dir.display());
            for dir in &g.cloud_saves {
                println!("Cloud saves: {}", dir.display());
            }
            print_gate(&g)?;
        }
        Command::Import { path } => {
            let r = import_path(&path, &game()?, &lib)?;
            let what = if r.legacy { "legacy mod" } else { "Obsidian mod" };
            let how = match r.outcome {
                InstallOutcome::Added => "added".to_string(),
                InstallOutcome::Replaced { old_version } => format!("replaced version {old_version}"),
            };
            println!("Imported {} ({what}, {how})", r.id);
            if r.legacy {
                println!("  {} changed, {} added resources", r.changed, r.added);
                if r.removed_ignored > 0 {
                    println!("  {} resources missing from the mod were left as in the game", r.removed_ignored);
                }
            }
            profile.sync(&lib)?;
            profile.save(&data)?;
        }
        Command::Updates => {
            let checked = updates::check(&GitHub::default(), &lib.list()?);
            if checked.is_empty() {
                println!("No mod names a GitHub repository.");
            }
            for c in checked {
                match c.result {
                    Ok(Some(v)) => println!("{}: version {v} is available (run obsidian update {})", c.id, c.id),
                    Ok(None) => println!("{}: up to date", c.id),
                    Err(e) => println!("{}: {e}", c.id),
                }
            }
        }
        Command::Update { ids } => {
            let github = GitHub::default();
            let mods = lib.list()?;
            let mut todo = ids;
            if todo.is_empty() {
                for c in updates::check(&github, &mods) {
                    match c.result {
                        Ok(Some(_)) => todo.push(c.id),
                        Ok(None) => {}
                        Err(e) => eprintln!("{}: {e}", c.id),
                    }
                }
            }
            if todo.is_empty() {
                println!("Nothing to update.");
            }
            for id in &todo {
                let m = mods.iter().find(|m| m.id == *id).ok_or_else(|| ObsidianError::ModMissing(id.clone()))?;
                let staged = updates::stage(&github, m)?;
                staged.install(&lib)?;
                println!("Updated {id} {} -> {} (run obsidian deploy)", m.version, staged.manifest.version);
            }
            profile.sync(&lib)?;
            profile.save(&data)?;
        }
        Command::List => {
            let mods = lib.list()?;
            if profile.mods.is_empty() {
                println!("The library is empty. Add mods with: obsidian import <folder or .zip>");
            }
            for (i, entry) in profile.mods.iter().enumerate() {
                let Some(m) = mods.iter().find(|m| m.id == entry.id) else { continue };
                let on = if entry.enabled { "on " } else { "off" };
                let scope = match m.scope {
                    Scope::Client => "client",
                    Scope::Session => "session",
                    Scope::Library => "library",
                };
                println!("{:>3}. [{on}] {} {} ({scope})", i + 1, m.id, m.version);
            }
        }
        Command::Order { ids } => {
            profile.set_order(&ids);
            profile.save(&data)?;
            println!("Load order: {}", profile.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(", "));
        }
        Command::Enable { id } => toggle(&mut profile, &lib, &data, &id, true)?,
        Command::Disable { id } => toggle(&mut profile, &lib, &data, &id, false)?,
        Command::Conflicts => {
            let c = obsidian_core::compose::plan(&lib, &profile)?;
            if c.conflicts.is_empty() {
                println!("No conflicts between the enabled mods.");
            }
            for x in c.conflicts {
                println!(
                    "{} {} in {}: {} wins over {}",
                    format_type(x.type_hash),
                    format_name(x.name_hash),
                    x.bundle,
                    x.winner,
                    x.losers.join(", ")
                );
            }
        }
        Command::Deploy => {
            obsidian_core::game::ensure_closed()?;
            let r = deploy(&game()?, &data, &DeployOptions::default())?;
            for p in &r.patches {
                println!("Wrote {}", p.display());
            }
            for p in &r.foreign {
                println!("Left in place (another tool): {}", p.display());
            }
            if let Some(b) = &r.backup {
                println!("Saves backed up to {}", b.display());
            }
            if !r.conflicts.is_empty() {
                println!("{} conflicts resolved by load order (see: obsidian conflicts)", r.conflicts.len());
            }
            if r.patches.is_empty() {
                println!("No mods enabled, so the game is vanilla.");
            }
        }
        Command::Purge => {
            obsidian_core::game::ensure_closed()?;
            let r = purge(&game()?, &data)?;
            for p in &r.removed {
                println!("Removed {}", p.display());
            }
            for p in &r.kept_changed {
                println!("Kept {} (changed since Obsidian wrote the file)", p.display());
            }
            if r.removed.is_empty() && r.kept_changed.is_empty() {
                println!("Nothing to purge.");
            }
        }
        Command::Status => {
            let g = game()?;
            print_gate(&g)?;
            let s = status(&g, &data, KNOWN_BUILDS)?;
            match &s.deployed {
                Some(d) if !d.files.is_empty() => println!("Deployed: {} ({} patches, {})", d.mods.join(", "), d.files.len(), d.time),
                _ => println!("Deployed: nothing, the game is vanilla"),
            }
            // Status must work with a damaged library, so a failure here only skips the line.
            if let Ok(w) = Profile::load(&data).and_then(|p| changes_waiting(&lib, &p, &data, Some(&g.contents()))) {
                println!("Changes waiting: {}", if w == ChangesWaiting::Waiting { "yes (run obsidian deploy)" } else { "no" });
            }
            if obsidian_core::game::is_running() {
                println!("The game is running.");
            }
            for p in &s.foreign {
                println!("Patch from another tool: {}", p.display());
            }
            if let Some(id) = &s.crash_culprit {
                println!("The last game launch stopped while loading {id}. If the game crashed, disable that mod and deploy again.");
            }
        }
        Command::Consent { state } => {
            profile.consent = matches!(state, OnOff::On);
            live_change(&profile, &data, &lib, &game()?, &format!("Session mods from other Obsidian users are now {}", if profile.consent { "allowed" } else { "not allowed" }))?;
        }
        Command::Set { id, option, value } => {
            // 7, true and "night" all work; anything that isn't JSON is taken as text.
            let parsed = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value.clone()));
            match obsidian_core::edit::set_option(&lib, &mut profile, &id, &option, Some(&parsed))? {
                obsidian_core::edit::Applies::AfterDeploy => {
                    profile.save(&data)?;
                    println!("{id} {option} is now {parsed}. Run obsidian deploy to apply.");
                }
                obsidian_core::edit::Applies::NextLaunch => live_change(&profile, &data, &lib, &game()?, &format!("{id} {option} is now {parsed}"))?,
            }
        }
        Command::Remove { id } => {
            lib.remove(&id)?;
            profile.sync(&lib)?;
            live_change(&profile, &data, &lib, &game()?, &format!("Removed {id}"))?;
        }
        Command::RestoreStarter => {
            starter::restore(&lib, &mut profile)?;
            live_change(&profile, &data, &lib, &game()?, "The Obsidian Kit is back as it came")?;
        }
        Command::Key { id, keybind, key } => {
            obsidian_core::edit::set_key(&lib, &mut profile, &id, &keybind, Some(&key))?;
            live_change(&profile, &data, &lib, &game()?, &format!("{id} {keybind} is now {key}"))?;
        }
        Command::Profile { action } => match action {
            ProfileAction::List => {
                let active = profiles::active(&data)?;
                for name in profiles::list(&data)? {
                    println!("{} {name}", if name == active { "*" } else { " " });
                }
            }
            ProfileAction::Use { name } => {
                profiles::use_profile(&data, &name)?;
                let mut p = Profile::load(&data)?;
                p.sync(&lib)?;
                live_change(&p, &data, &lib, &game()?, &format!("Profile {} is active", profiles::active(&data)?))?;
            }
            ProfileAction::New { name, copy } => {
                profiles::create(&data, &name, copy.as_deref())?;
                println!("Created profile {name}. Switch to it with: obsidian profile use {name}");
            }
        },
    }
    Ok(())
}

/// Saves the profile and rewrites the settings file the loader reads at launch, so the change
/// needs no redeploy once the loader is in the game.
fn live_change(profile: &Profile, data: &std::path::Path, lib: &Library, g: &GameInstall, what: &str) -> Result<()> {
    profile.save(data)?;
    obsidian_core::edit::write_settings(lib, profile, &g.saves_dir)?;
    // The loader lives in the boot bundle patch, so a deploy that wrote none has no loader in the game.
    let loader_deployed = status(g, data, KNOWN_BUILDS)
        .ok()
        .and_then(|s| s.deployed)
        .is_some_and(|d| d.files.iter().any(|f| f.path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(obsidian_core::loader::BUNDLE))));
    if loader_deployed {
        println!("{what}. Applies at the next game launch.");
    } else {
        println!("{what}. Run obsidian deploy to apply.");
    }
    Ok(())
}

fn toggle(profile: &mut Profile, lib: &Library, data: &std::path::Path, id: &str, on: bool) -> Result<()> {
    let others = profile.switch(lib, id, on)?;
    profile.save(data)?;
    let word = if on { "on" } else { "off" };
    println!("{id} is now {word}. Run obsidian deploy to apply.");
    for other in others {
        println!("{other} is now {word} too, because {} needs it.", if on { id } else { &other });
    }
    Ok(())
}

fn print_gate(g: &GameInstall) -> Result<()> {
    let r = obsidian_core::gate::check_build(&g.root, KNOWN_BUILDS)?;
    match r.build {
        Some(b) => println!("Game build: {b}"),
        None => println!("Game build: not recognised (differs in {})", r.mismatches.join(", ")),
    }
    Ok(())
}
