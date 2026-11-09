//! `obsidian`: the command line over obsidian-core.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use obsidian_core::deploy::{ChangesWaiting, DeployOptions, changes_waiting, deploy, purge, status};
use obsidian_core::game::{GameInstall, find_game};
use obsidian_core::gate::KNOWN_BUILDS;
use obsidian_core::hash::{display_name, display_text, format_type};
use obsidian_core::import::import_path;
use obsidian_core::library::{InstallOutcome, Library, Profile};
use obsidian_core::paths::data_dir;
use obsidian_core::{profiles, starter};
use obsidian_core::updates::{self, GitHub};
use obsidian_core::{ObsidianError, Result};

#[derive(Parser)]
#[command(name = "obsidian", version, about = "Launcher and mod loader for HELLDIVERS (2015)")]
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
    /// Block session mods from others in a public lobby.
    BlockPublic { state: OnOff },
    /// Keep Steam achievements from unlocking (on or off), from the next launch
    BlockAchievements { state: OnOff },
    /// Keep the account's progression as it is (on or off), from the next launch
    FreezeProgress { state: OnOff },
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
    /// List and manage saves (Official and Sandbox)
    Saves {
        #[command(subcommand)]
        action: SavesAction,
    },
    /// Custom Galactic Wars: the Commander package, hosting, and the war the next launch plays
    Commander {
        #[command(subcommand)]
        action: CommanderAction,
    },
}

#[derive(Subcommand)]
enum CommanderAction {
    /// Show the package, the wars this PC plays and the one the next launch plays
    Status,
    /// Install a Commander package (.zip or folder)
    Install { path: PathBuf },
    /// Remove the Commander package; hosted wars stay in the data folder
    Uninstall,
    /// Host: run the war server and the relay until Enter is pressed or input ends
    Serve,
    /// Play a war on the next launch: a war hosted here, or with --lan http://<address>:<port>
    Play {
        id: String,
        #[arg(long, default_value = "Custom War")]
        name: String,
        #[arg(long)]
        lan: Option<String>,
    },
    /// Play the official war on the next launch
    Official,
}

#[derive(Subcommand)]
enum SavesAction {
    /// List the saves of the active Steam account; the one the game boots is marked with *
    List,
    /// Select a save: an Official save is restored, a Sandbox only selected
    Activate { id: String },
    /// Copy an Official save into a new Official save
    Snapshot { id: String, name: String },
    /// A new blank Official save, in the library until activated
    NewOfficial { name: String },
    /// A new blank Sandbox save
    NewSandbox { name: String },
    /// A new Sandbox save copied from any save
    Duplicate { id: String, name: String },
    /// Rename a Sandbox save
    Rename { id: String, name: String },
    /// Delete a Sandbox save
    Delete { id: String },
    /// Write a Sandbox save to a zip
    Export { id: String, dest: PathBuf },
    /// Add a save from a zip, always as a Sandbox
    Import { path: PathBuf },
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
            ExitCode::FAILURE
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
            | Command::BlockPublic { .. } | Command::BlockAchievements { .. } | Command::FreezeProgress { .. } | Command::Set { .. } | Command::Key { .. } | Command::Profile { .. }
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
                let scope = m.scope.name();
                println!("{:>3}. [{on}] {} {} ({scope})", i + 1, m.id, m.version);
            }
            print_broken(&lib)?;
        }
        Command::Order { ids } => {
            profile.set_order(&ids);
            profile.save(&data)?;
            println!("Load order: {}", profile.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(", "));
        }
        Command::Enable { id } => toggle(&mut profile, &lib, &data, &id, true)?,
        Command::Disable { id } => toggle(&mut profile, &lib, &data, &id, false)?,
        Command::Conflicts => {
            print_broken(&lib)?;
            let c = obsidian_core::compose::plan(&lib, &profile)?;
            if c.conflicts.is_empty() {
                println!("No conflicts between the enabled mods.");
            }
            for x in c.conflicts {
                println!(
                    "{} {} in {}: {} wins over {}",
                    format_type(x.type_hash),
                    display_name(x.name_hash),
                    display_text(&x.bundle),
                    x.winner,
                    x.losers.join(", ")
                );
            }
        }
        Command::Deploy => {
            obsidian_core::game::ensure_closed()?;
            print_broken(&lib)?;
            let r = deploy(&game()?, &data, &DeployOptions::default())?;
            if r.unchanged {
                println!("The game already has these mods, so nothing was written.");
            }
            for p in r.patches.iter().filter(|_| !r.unchanged) {
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
        Command::BlockPublic { state } => {
            profile.block_public = matches!(state, OnOff::On);
            let now = if profile.block_public { "blocked in public lobbies" } else { "allowed" };
            live_change(&profile, &data, &lib, &game()?, &format!("Session mods from other players are now {now}"))?;
        }
        Command::BlockAchievements { state } => {
            profile.block_achievements = matches!(state, OnOff::On);
            let now = if profile.block_achievements { "kept from unlocking" } else { "unlocking as usual" };
            live_change(&profile, &data, &lib, &game()?, &format!("Achievements are {now} from the next launch"))?;
        }
        Command::FreezeProgress { state } => {
            profile.freeze_progress = matches!(state, OnOff::On);
            let now = if profile.freeze_progress { "frozen" } else { "saved as usual" };
            live_change(&profile, &data, &lib, &game()?, &format!("Your progression is {now} from the next launch"))?;
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
        Command::Saves { action } => run_saves(action, &data, &game()?)?,
        Command::Commander { action } => run_commander(action, &data, game)?,
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

fn run_commander(action: CommanderAction, data: &std::path::Path, game: impl Fn() -> Result<GameInstall>) -> Result<()> {
    use obsidian_core::commander::{self, State, WarHost, WarRef, package, relay::Relay, server};
    let saves = || -> Result<obsidian_core::saves::Saves> { obsidian_core::saves::Saves::open(data, &game()?, obsidian_core::game::is_running) };
    match action {
        CommanderAction::Status => {
            let s = State::load(data);
            match package::installed(data) {
                Some(i) => println!("Commander {} (protocol {})", i.version, i.protocol),
                None => println!("Commander isn't installed"),
            }
            for w in &s.wars {
                let mark = if s.playing.as_deref() == Some(w.id.as_str()) { "*" } else { " " };
                let host = match w.host {
                    WarHost::Local => "this PC".to_string(),
                    WarHost::Lan => w.base.clone().unwrap_or_default(),
                };
                println!("{mark} {} {} ({host})", w.id, w.name);
            }
            if s.playing.is_none() {
                println!("The next launch plays the official war.");
            }
        }
        CommanderAction::Install { path } => {
            let i = package::install(data, &path)?;
            println!("Commander {} installed", i.version);
        }
        CommanderAction::Uninstall => {
            package::uninstall(data)?;
            println!("Commander removed");
        }
        CommanderAction::Serve => {
            let srv = server::start(data)?;
            let relay = Relay::start(commander::ensure_relay_port(data)?)?;
            println!("War server: http://127.0.0.1:{}/?token={}", srv.port, srv.token);
            println!("Relay: port {}. Press Enter to stop.", relay.port);
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = stop.clone();
            std::thread::spawn(move || {
                let mut line = String::new();
                let _ = std::io::stdin().read_line(&mut line);
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            // The war played can change from the app or another CLI call: follow it.
            while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                relay.set_route(State::load(data).route());
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        CommanderAction::Play { id, name, lan } => {
            let war = match lan {
                Some(base) => WarRef { id: id.clone(), name, host: WarHost::Lan, base: Some(base.trim_end_matches('/').to_string()), save: None, computer: None },
                None => WarRef { id: id.clone(), name, host: WarHost::Local, base: None, save: None, computer: None },
            };
            commander::play_war(data, &saves()?, war)?;
            let s = State::load(data);
            println!("The next launch plays war {id} on Sandbox save {}.", s.war(&id).and_then(|w| w.save.clone()).unwrap_or_default());
            if s.war(&id).is_some_and(|w| w.host == WarHost::Local) {
                println!("Keep `obsidian commander serve` (or the app) running while playing.");
            }
        }
        CommanderAction::Official => {
            commander::play_official(data, &saves()?)?;
            println!("The next launch plays the official war.");
        }
    }
    Ok(())
}

fn run_saves(action: SavesAction, data: &std::path::Path, g: &GameInstall) -> Result<()> {
    use obsidian_core::saves::Saves;
    let s = Saves::open(data, g, obsidian_core::game::is_running)?;
    let show_id = |id: String| println!("Saved as {id}");
    match action {
        SavesAction::List => {
            let list = s.list()?;
            println!("Steam account {}", list.account);
            for row in list.saves {
                let mark = if row.active { "*" } else { " " };
                let kind = match row.kind {
                    obsidian_core::saves::SaveKind::Official => "official",
                    obsidian_core::saves::SaveKind::Sandbox => "sandbox ",
                };
                let stats = row.stats.map_or_else(|| row.error.unwrap_or_default(), |t| format!("rank {} xp {:.0} samples {:.0}", t.rank, t.xp, t.samples));
                println!("{mark} {} {kind} {} ({stats})", row.id, row.name);
            }
        }
        SavesAction::Activate { id } => {
            s.activate(&id)?;
            println!("{id} is now the save the game boots.");
        }
        SavesAction::Snapshot { id, name } => show_id(s.snapshot(&id, &name)?),
        SavesAction::NewOfficial { name } => show_id(s.create_official(&name)?),
        SavesAction::NewSandbox { name } => show_id(s.create_sandbox(&name)?),
        SavesAction::Duplicate { id, name } => show_id(s.duplicate(&id, &name)?),
        SavesAction::Rename { id, name } => s.rename(&id, &name)?,
        SavesAction::Delete { id } => s.delete(&id)?,
        SavesAction::Export { id, dest } => s.export(&id, &dest)?,
        SavesAction::Import { path } => show_id(s.import(&path, None)?),
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

/// The mod folders that can't be read: they stay out of deploys until they're fixed or removed, and
/// saying nothing would leave a mod out without a word.
fn print_broken(lib: &Library) -> Result<()> {
    for b in lib.broken()? {
        println!("Left out, can't be read: {} ({}). Fix it or remove it: obsidian remove {}", b.folder, b.reason, b.folder);
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
