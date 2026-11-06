//! obsidian-core: everything Obsidian does to game files.

pub mod backup;
pub mod bank;
pub mod bundle;
pub mod compose;
pub mod deploy;
pub mod edit;
pub mod folders;
pub mod error;
pub mod game;
pub mod game_options;
pub mod gate;
pub mod hash;
pub mod images;
pub mod import;
pub mod library;
pub mod loader;
pub mod lua;
pub mod luajit;
pub mod manifest;
pub mod modset;
pub mod options;
pub mod overlay;
pub mod paths;
pub mod profile_zip;
pub mod profiles;
pub mod report;
pub mod settings;
pub mod sort;
pub mod starter;
#[cfg(test)]
pub(crate) mod testutil;
pub mod updates;
pub mod vdf;

pub use error::{ObsidianError, Result};
