//! Errors returned by obsidian-core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsidianError {
    #[error("not a resource name or #hash: {0}")]
    BadRef(String),
    #[error("unexpected bundle version {0:#x}")]
    BadBundleVersion(u32),
    #[error("bundle is truncated")]
    TruncatedBundle,
    #[error("bundle has unexpected data after the last entry")]
    TrailingData,
    #[error("invalid Lua bytecode: {0}")]
    InvalidBytecode(String),
    #[error("HELLDIVERS was not found in these Steam libraries: {0}")]
    GameNotFound(String),
    #[error("obsidian.json, {field}: {reason}")]
    BadManifest { field: String, reason: String },
    #[error("archive type not supported yet (use a folder or a .zip): {0}")]
    UnsupportedArchive(String),
    #[error("nothing to import: no obsidian.json and no game bundle files in {0}")]
    NothingToImport(String),
    #[error("{0} is not a bundle of this game")]
    UnknownBundle(String),
    #[error("{0} is a ReShade preset, not a game mod: it works with ReShade installed in the game folder, outside Obsidian")]
    ReshadePreset(String),
    #[error("the game files don't match a build Obsidian knows ({}), so it can't tell what this mod changes. If you copied a mod into the game folder by hand, verify the game files in Steam and import it again", .0.mismatches.join(", "))]
    GameFilesChanged(crate::gate::GateReport),
    #[error("bundle {bundle}, entry {entry}: its streamed data is in neither the mod's nor the game's .stream file")]
    LegacyStreamMissing { bundle: String, entry: String },
    #[error("mod {0} is in the profile but not in the library")]
    ModMissing(String),
    #[error("{mod_id} needs {requires}, which is not switched on")]
    MissingRequirement { mod_id: String, requires: String },
    #[error("{mod_id} needs {requires}, which is not installed")]
    RequirementNotInstalled { mod_id: String, requires: String },
    #[error("the LuaJIT 2.0 compiler was not found (looked in {0}); run tools/build-luajit.cmd or set OBSIDIAN_LUAJIT")]
    CompilerUnavailable(String),
    #[error("Lua error in {chunk}: {message}")]
    CompileFailed { chunk: String, message: String },
    #[error("option {option}: {value} is not an allowed value")]
    BadOptionValue { option: String, value: String },
    #[error("mod {mod_id}: {name} belongs to Obsidian Loader; use \"entry\" to run a script at boot")]
    ReservedResource { mod_id: String, name: String },
    #[error("Obsidian Loader files were not found (looked in {0})")]
    LoaderMissing(String),
    #[error("this copy of Obsidian has no Obsidian Kit to restore")]
    NoStarterPack,
    #[error("{mod_id} has no keybind {keybind}")]
    UnknownKeybind { mod_id: String, keybind: String },
    #[error("{0:?} can't be a profile name: use 1 to 40 characters, none of \\ / : * ? \" < > |")]
    ProfileName(String),
    #[error("a profile named {0} already exists")]
    ProfileExists(String),
    #[error("there is no profile named {0}")]
    ProfileMissing(String),
    #[error("the last profile can't be deleted")]
    LastProfile,
    #[error("mod {mod_id} has no option {option}")]
    UnknownOption { mod_id: String, option: String },
    #[error("mod {mod_id}: every resource needs a bundle in this version")]
    BundleRequired { mod_id: String },
    #[error("a sound bank can't be read: {0}")]
    BadBank(String),
    #[error("mod {mod_id} changes sound bank {bank}, and this game doesn't have it")]
    RecipeBank { mod_id: String, bank: String },
    #[error("mod {mod_id} changes sound bank {bank} in language {lang:#x}, and the bank doesn't have that language")]
    RecipeLanguage { mod_id: String, bank: String, lang: u32 },
    #[error("mod {mod_id} changes clip {clip} of sound bank {bank}, and the bank doesn't have it in language {lang:#x}")]
    RecipeClip { mod_id: String, bank: String, clip: String, lang: u32 },
    #[error("mod {mod_id}: {file} can't go in a sound bank: {why}")]
    RecipeFile { mod_id: String, file: String, why: String },
    #[error("the patch for bundle {0} would pass 4 GB, which a bundle can't hold. Turn off some of the mods that change it")]
    PatchTooLarge(String),
    #[error("game build not recognised: {}", .0.mismatches.join(", "))]
    UnsupportedBuild(crate::gate::GateReport),
    #[error("bundle {bundle}: patch_{missing} is missing below another tool's patches, so the game would skip them")]
    ForeignPatchGap { bundle: String, missing: u32 },
    #[error("{0} is not an image Obsidian can use: PNG, JPG or WebP, up to 4 MB")]
    BadImage(String),
    #[error("the folder is locked")]
    LockedFolder,
    #[error("close the game first")]
    GameRunning,
    #[error("another deploy or purge is running on this library; try again when it ends")]
    Busy,
    #[error("deploy failed and was undone: {0}")]
    DeployFailed(String),
    #[error("purge couldn't remove a file, so run it again with the game closed: {0}")]
    PurgeStuck(String),
    #[error("the game has no Proton prefix yet: start it once through Steam, then try again")]
    NoProtonPrefix,
    #[error("{0} names no GitHub repository to update from")]
    NoUpdateSource(String),
    #[error("{0} has no release with a version number")]
    NoRelease(String),
    #[error("the latest release of {0} has no single .zip with a mod to install")]
    NoUpdateAsset(String),
    #[error("the latest release holds {found}, not a newer {wanted}")]
    UpdateMismatch { wanted: String, found: String },
    #[error("GitHub could not be reached: {0}")]
    UpdateNetwork(String),
    #[error("GitHub's limit on update checks was reached, so try again in about an hour")]
    UpdateLimited,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ObsidianError>;
