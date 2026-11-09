//! Turning the enabled mods of a profile into one resource set per game bundle.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::library::{Library, Profile};
use crate::lua::{check_bytecode, wrap_resource};
use crate::luajit::compile;
use crate::hash::hash_name;
use crate::manifest::{Manifest, Payload, load_manifest};
use crate::options::{effective_values, when_matches};
use crate::{ObsidianError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub type_hash: u64,
    pub name_hash: u64,
    pub flags: u32,
    /// Empty in a composition from `plan`, which keeps only the digest.
    pub data: Vec<u8>,
    /// SHA-256 of the data the game gets.
    pub digest: [u8; 32],
    pub owner: String,
    /// A resource with language variants or streamed data, one piece per language; `data` is then
    /// empty. Without pieces the resource is `data` alone, for every language, with nothing streamed.
    pub pieces: Vec<Piece>,
    /// A bank rebuilt from the game at deploy; `data` and `pieces` are then empty until `rebuilt`.
    pub recipe: Option<Recipe>,
}

/// One clip a mod puts in a bank.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipFile {
    pub owner: String,
    pub data: PathBuf,
    pub stream: Option<PathBuf>,
    pub digest: [u8; 32],
}

/// What goes on top of a bank: the clips of the mods that change it, by (language, clip).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recipe {
    pub clips: BTreeMap<(u32, u64), ClipFile>,
    /// A whole-bank mod below the clips. Without one, they go on the game's bank.
    base: Option<Base>,
    /// (loser, winner) for each clip two mods give different bytes.
    lost: Vec<(String, String)>,
    /// The first bundle that holds the bank, which reports its conflicts and has its digest.
    first: String,
    /// Set on the resources the clips of every recipe make together (overlay.rs): the master, and
    /// Obsidian's bank of new clips beside each bank they replace. `clips` and `base` are then empty.
    overlay: Option<Part>,
}

/// One of the resources of the overlay: the master, or the bank of new clips for one source.
#[derive(Debug, Clone, PartialEq)]
enum Part {
    Master(std::sync::Arc<Overlay>),
    Bank(std::sync::Arc<Overlay>, usize),
}

/// The clips of every recipe, deployed as an overlay (overlay.rs): the master in the bundle that holds
/// it, and for each bank the clips replace, a bank of the new clips in every bundle that holds that bank.
#[derive(Debug, Clone, PartialEq)]
struct Overlay {
    /// The bundle with the game's master, and the master's name.
    bundle: String,
    master: u64,
    /// A whole master of a mod, which the new ids go into in place of the game's. Empty in a plan.
    master_base: Option<Vec<u8>>,
    /// The new clip for each replaced clip, by language: the later mod's when two give one.
    clips: BTreeMap<u64, BTreeMap<u32, ClipFile>>,
    /// The banks the clips are in, which give the clips in the languages no mod changes. A whole bank
    /// of a mod below the clips comes first.
    sources: Vec<Source>,
}

/// A bank the clips replace, and where Obsidian's bank of its new clips goes: the bundles that hold it.
/// The game loads those after it has set the language, so each language gets its own variant there; the
/// boot bundle loads before, and a resource there takes language 0 in every language.
#[derive(Debug, Clone, PartialEq)]
struct Source {
    bank: u64,
    bundles: Vec<String>,
    /// A whole bank of a mod below the clips, which stands for the game's; its pieces are empty in a plan.
    base: Option<([u8; 32], Vec<Piece>)>,
    /// What the recipes put in it, (language, clip, mod), which the bank must have.
    wanted: Vec<(u32, u64, String)>,
}

/// Set, a deploy rebuilds each changed sound bank whole in every bundle that holds it, as before the overlay.
pub const SOUND_REBUILD: &str = "OBSIDIAN_SOUND_REBUILD";

#[derive(Debug, Clone, PartialEq)]
struct Base {
    owner: String,
    digest: [u8; 32],
    /// Empty in a composition from `plan`.
    pieces: Vec<Piece>,
}

/// One language of a resource: the bytes in the patch and the bytes in the patch's `.stream`.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub lang: u32,
    pub data: Vec<u8>,
    pub stream: Vec<u8>,
}

impl Planned {
    pub fn new(type_hash: u64, name_hash: u64, flags: u32, data: Vec<u8>, owner: String) -> Planned {
        let digest = sha256(&data);
        Planned { type_hash, name_hash, flags, data, digest, owner, pieces: Vec::new(), recipe: None }
    }

    pub fn with_pieces(type_hash: u64, name_hash: u64, flags: u32, pieces: Vec<Piece>, owner: String) -> Planned {
        let digest = pieces_digest(&pieces);
        Planned { type_hash, name_hash, flags, data: Vec::new(), digest, owner, pieces, recipe: None }
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).into()
}

/// The digest of a resource in pieces: each piece's language and both lengths, then its bytes.
fn pieces_digest(pieces: &[Piece]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for p in pieces {
        h.update(p.lang.to_le_bytes());
        h.update((p.data.len() as u64).to_le_bytes());
        h.update((p.stream.len() as u64).to_le_bytes());
        h.update(&p.data);
        h.update(&p.stream);
    }
    h.finalize().into()
}

/// When a file last changed and its size.
type Stamp = (SystemTime, u64);
/// Digests of resource files by path, kept while the file is unchanged, so `plan` reads (and
/// compiles) a file only once.
type DigestCache = std::sync::Mutex<std::collections::HashMap<PathBuf, (Stamp, [u8; 32])>>;

fn digests() -> &'static DigestCache {
    static CACHE: std::sync::OnceLock<DigestCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The digests on disk, so a start doesn't read every file and compile every script again.
const DIGESTS: &str = "digests.json";

#[derive(Serialize, Deserialize)]
struct SavedDigest {
    path: PathBuf,
    secs: u64,
    nanos: u32,
    len: u64,
    digest: String,
}

#[derive(Serialize, Deserialize)]
struct SavedDigests {
    /// Another version may compile or wrap a script differently, so its digests are no good here.
    version: String,
    files: Vec<SavedDigest>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    if text.len() != 64 {
        return None;
    }
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// Reads the digests of a data folder, once. Losing the file only costs the time to work them out.
fn load_digests(data: &Path) {
    static LOADED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());
    let mut loaded = LOADED.lock().unwrap();
    if loaded.iter().any(|d| d == data) {
        return;
    }
    loaded.push(data.to_path_buf());
    let Ok(text) = std::fs::read_to_string(data.join(DIGESTS)) else { return };
    let Ok(saved) = serde_json::from_str::<SavedDigests>(&text) else { return };
    if saved.version != env!("CARGO_PKG_VERSION") {
        return;
    }
    let mut cache = digests().lock().unwrap();
    for f in saved.files {
        if let Some(digest) = unhex(&f.digest) {
            cache.insert(f.path, ((SystemTime::UNIX_EPOCH + Duration::new(f.secs, f.nanos), f.len), digest));
        }
    }
}

fn save_digests(data: &Path) {
    let files = digests()
        .lock()
        .unwrap()
        .iter()
        .filter(|(path, _)| path.starts_with(data))
        .filter_map(|(path, ((time, len), digest))| {
            let since = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
            Some(SavedDigest { path: path.clone(), secs: since.as_secs(), nanos: since.subsec_nanos(), len: *len, digest: hex(digest) })
        })
        .collect();
    let saved = SavedDigests { version: env!("CARGO_PKG_VERSION").to_string(), files };
    if let Ok(text) = serde_json::to_vec(&saved) {
        let _ = crate::paths::write_atomic(&data.join(DIGESTS), &text);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub bundle: String,
    pub type_hash: u64,
    pub name_hash: u64,
    pub winner: String,
    pub losers: Vec<String>,
}

/// An enabled mod, in load order, with the option values in force and how many resources joined.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposedMod {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub values: BTreeMap<String, Value>,
    pub resources: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Composition {
    pub bundles: BTreeMap<String, Vec<Planned>>,
    pub conflicts: Vec<Conflict>,
    pub mods: Vec<ComposedMod>,
    /// A plan's sound bank clips with no bank to go in, by mod. A deploy says why, and until then
    /// there's always one waiting.
    pub unplaced: Vec<String>,
    /// A Sandbox save is selected, so the loader has to be in the game.
    pub sandbox: bool,
}

/// Everything the game would get, with the data, for a deploy. `contents` is the game's folder of
/// bundles, which a mod's sound bank clips need.
pub fn compose_for(lib: &Library, profile: &Profile, contents: Option<&Path>) -> Result<Composition> {
    compose_with(lib, profile, true, contents, None, None)
}

/// Told each enabled mod as a compose reaches it: the place in the list, how many, the mod.
pub type OnMod<'a> = &'a dyn Fn(usize, usize, &Manifest);

/// `compose_for`, saying which mod it is reading.
pub fn compose_reporting(lib: &Library, profile: &Profile, contents: Option<&Path>, on_mod: OnMod<'_>) -> Result<Composition> {
    compose_with(lib, profile, true, contents, None, Some(on_mod))
}

/// `compose_for` with no game.
pub fn compose(lib: &Library, profile: &Profile) -> Result<Composition> {
    compose_for(lib, profile, None)
}

/// `compose` without the data: the conflicts and a digest per resource, enough to tell whether a
/// deploy is waiting. Unchanged files are not read again, which keeps the window quick with a
/// large library. Sound bank clips are left out when the game or the bank can't be found: the deploy
/// is what says so.
pub fn plan_for(lib: &Library, profile: &Profile, contents: Option<&Path>) -> Result<Composition> {
    compose_with(lib, profile, false, contents, None, None)
}

/// `plan_for` with the library's manifests already read, by id, so they aren't read again: the
/// window's snapshot has them, and it runs after every click.
pub fn plan_with(lib: &Library, profile: &Profile, contents: Option<&Path>, manifests: &BTreeMap<String, Manifest>) -> Result<Composition> {
    compose_with(lib, profile, false, contents, Some(manifests), None)
}

/// `plan_for` with no game.
pub fn plan(lib: &Library, profile: &Profile) -> Result<Composition> {
    plan_for(lib, profile, None)
}

/// A digest of a file, kept while the file is unchanged.
fn file_digest(path: &Path, stamps: &std::collections::HashMap<PathBuf, Stamp>, worked_out: &mut bool) -> Result<[u8; 32]> {
    let stamp = match stamps.get(path) {
        Some(s) => *s,
        None => {
            let meta = std::fs::metadata(path)?;
            (meta.modified()?, meta.len())
        }
    };
    if let Some((_, d)) = digests().lock().unwrap().get(path).filter(|(s, _)| *s == stamp) {
        return Ok(*d);
    }
    let d = sha256(&std::fs::read(path)?);
    digests().lock().unwrap().insert(path.to_path_buf(), (stamp, d));
    *worked_out = true;
    Ok(d)
}

/// `file_digest` for a file outside a mod's folder walk, such as the loader's: worked out once, then
/// kept while its time and size stay the same.
pub(crate) fn cached_digest(path: &Path) -> Result<[u8; 32]> {
    file_digest(path, &Default::default(), &mut false)
}

fn file_stamp(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

fn stream_of(bundle: &Path) -> PathBuf {
    let mut name = bundle.as_os_str().to_owned();
    name.push(".stream");
    PathBuf::from(name)
}

/// Which bundles hold which resources, by (type, name).
type Holders = std::collections::HashMap<(u64, u64), Vec<String>>;
/// The last answer for a game folder, and the state of its bundles then.
type HoldersCache = std::sync::Mutex<std::collections::HashMap<PathBuf, (Vec<(String, Stamp)>, std::sync::Arc<Holders>)>>;

/// The bundles of a game that hold each resource, from the start of each file. The game's own bundles
/// only: its name has no `.patch_N` or `.stream`.
fn holders(contents: &Path) -> std::sync::Arc<Holders> {
    static CACHE: std::sync::OnceLock<HoldersCache> = std::sync::OnceLock::new();
    let mut state: Vec<(String, Stamp)> = std::fs::read_dir(contents)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let hex = name.len() == 16 && name.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if !hex {
                return None;
            }
            let meta = e.metadata().ok()?;
            Some((name, (meta.modified().ok()?, meta.len())))
        })
        .collect();
    state.sort();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((was, found)) = cache.lock().unwrap().get(contents) {
        if *was == state {
            return found.clone();
        }
    }
    let mut found = Holders::new();
    for (name, _) in &state {
        for resource in crate::bundle::read_index(&contents.join(name)).unwrap_or_default() {
            found.entry(resource).or_default().push(name.clone());
        }
    }
    let found = std::sync::Arc::new(found);
    cache.lock().unwrap().insert(contents.to_path_buf(), (state, found.clone()));
    found
}

/// The bytes of one resource as the game has it, one language at a time, with its flags.
fn game_resource(contents: &Path, bundle: &str, type_hash: u64, name_hash: u64, mut each: impl FnMut(Piece) -> Result<()>) -> Result<u32> {
    use std::io::{Read, Seek, SeekFrom};
    let path = contents.join(bundle);
    let mut reader = crate::bundle::Reader::open(&path)?;
    let heads = reader.heads()?;
    let entry = heads.iter().find(|e| (e.type_hash, e.name_hash) == (type_hash, name_hash)).ok_or(ObsidianError::TruncatedBundle)?;
    let mut stream_file = None;
    for v in &entry.variants {
        let data = reader.read(v.at, v.size as usize)?;
        let mut stream = vec![0u8; v.stream_size as usize];
        if v.stream_size > 0 {
            if stream_file.is_none() {
                stream_file = Some(std::fs::File::open(stream_of(&path))?);
            }
            let f = stream_file.as_mut().unwrap();
            f.seek(SeekFrom::Start(v.stream_at))?;
            f.read_exact(&mut stream)?;
        }
        each(Piece { lang: v.lang, data, stream })?;
    }
    Ok(entry.flags)
}

/// A digest for a bank as the game has it, from when its bundle and stream last changed. A bank can pass a
/// gigabyte and this runs after every click, and the game's files change only with a game update.
fn game_digest(contents: &Path, bundle: &str, name_hash: u64) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let path = contents.join(bundle);
    let (time, len) = file_stamp(&path).ok_or(ObsidianError::TruncatedBundle)?;
    let stamp = match file_stamp(&stream_of(&path)) {
        Some((t, n)) => (time.max(t), len + n),
        None => (time, len),
    };
    let since = stamp.0.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(bundle.as_bytes());
    h.update(name_hash.to_le_bytes());
    h.update(since.as_nanos().to_le_bytes());
    h.update(stamp.1.to_le_bytes());
    Ok(h.finalize().into())
}

fn recipe_digest(base: &[u8; 32], recipe: &Recipe) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(base);
    for ((lang, clip), file) in &recipe.clips {
        h.update(lang.to_le_bytes());
        h.update(clip.to_le_bytes());
        h.update(file.digest);
    }
    h.finalize().into()
}

/// The bank a recipe makes, for one bundle: the base with its clips, every other language as it was.
/// Reads it from the game here, so a deploy holds only the bundle it's writing.
pub fn rebuilt(contents: &Path, bundle: &str, planned: Planned) -> Result<Planned> {
    let Some(recipe) = &planned.recipe else { return Ok(planned) };
    if let Some(part) = &recipe.overlay {
        let (Part::Master(o) | Part::Bank(o, _)) = part;
        let (master, flags, mut banks) = overlay_made(contents, o)?;
        let mut out = match part {
            Part::Master(_) => Planned::new(planned.type_hash, planned.name_hash, flags, master, planned.owner),
            Part::Bank(_, i) => Planned::with_pieces(planned.type_hash, planned.name_hash, 0, std::mem::take(&mut banks[*i]), planned.owner),
        };
        out.digest = planned.digest;
        return Ok(out);
    }
    let bank = format!("#{:016x}", planned.name_hash);
    let mut flags = planned.flags;
    let pieces_in = match &recipe.base {
        Some(base) => base.pieces.clone(),
        None => {
            let mut pieces = Vec::new();
            flags = game_resource(contents, bundle, planned.type_hash, planned.name_hash, |p| {
                pieces.push(p);
                Ok(())
            })?;
            pieces
        }
    };
    let mut pieces = Vec::with_capacity(pieces_in.len());
    if let Some(((lang, _), file)) = recipe.clips.iter().find(|((l, _), _)| !pieces_in.iter().any(|p| p.lang == *l)) {
        return Err(ObsidianError::RecipeLanguage { mod_id: file.owner.clone(), bank, lang: *lang });
    }
    for piece in pieces_in {
        let mine: Vec<_> = recipe.clips.iter().filter(|((l, _), _)| *l == piece.lang).collect();
        if mine.is_empty() {
            pieces.push(piece);
            continue;
        }
        let rows = crate::bank::rows_of(&piece.data)?;
        let mut files = Vec::with_capacity(mine.len());
        for ((lang, clip), file) in &mine {
            if !rows.iter().any(|r| r.clip == *clip) {
                return Err(ObsidianError::RecipeClip { mod_id: file.owner.clone(), bank, clip: format!("#{clip:016x}"), lang: *lang });
            }
            let data = std::fs::read(&file.data)?;
            let name = || file.data.file_name().unwrap_or_default().to_string_lossy().into_owned();
            crate::bank::check_clip(&data).map_err(|why| ObsidianError::RecipeFile { mod_id: file.owner.clone(), file: name(), why })?;
            let stream = match &file.stream {
                Some(s) => std::fs::read(s)?,
                None => Vec::new(),
            };
            files.push((*clip, data, stream));
        }
        let new = files.iter().map(|(clip, d, s)| (*clip, (d.as_slice(), s.as_slice()))).collect();
        let (data, stream) = crate::bank::rebuild(&piece.data, &piece.stream, &new)?;
        pieces.push(Piece { lang: piece.lang, data, stream });
    }
    let mut out = Planned::with_pieces(planned.type_hash, planned.name_hash, flags, pieces, planned.owner);
    out.digest = planned.digest;
    Ok(out)
}

/// A bank's clips by language: the ids of every row, and the bytes (in the bank, in the stream) of the
/// rows wanted.
type HeldClips = BTreeMap<u32, (BTreeSet<u64>, BTreeMap<u64, crate::overlay::ClipBytes>)>;

/// The clips of a bank as the game has it, reading only their rows: a bank in 9 languages can pass a
/// gigabyte, and the overlay needs a few clips of it.
fn game_clips(contents: &Path, bundle: &str, bank: u64, wanted: &BTreeSet<u64>) -> Result<HeldClips> {
    use std::io::{Read, Seek, SeekFrom};
    let path = contents.join(bundle);
    let mut reader = crate::bundle::Reader::open(&path)?;
    let heads = reader.heads()?;
    let entry = heads.iter().find(|e| (e.type_hash, e.name_hash) == (hash_name("timpani_bank"), bank)).ok_or(ObsidianError::TruncatedBundle)?;
    let bad = || ObsidianError::BadBank(format!("#{bank:016x} has a row past its end"));
    let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
    let mut stream_file = None;
    let mut out = HeldClips::new();
    for v in &entry.variants {
        let count = word(&reader.read(v.at, 4)?, 0);
        if 8 + 24 * count as u64 > v.size as u64 {
            return Err(bad());
        }
        let table = reader.read(v.at + 8, 24 * count)?;
        let (ids, clips) = out.entry(v.lang).or_default();
        for row in table.chunks(24) {
            let clip = u64::from_le_bytes(row[..8].try_into().unwrap());
            ids.insert(clip);
            if !wanted.contains(&clip) {
                continue;
            }
            let (off, size, soff, ssize) = (word(row, 8), word(row, 12), word(row, 16), word(row, 20));
            if off + size > v.size as usize || soff + ssize > v.stream_size as usize {
                return Err(bad());
            }
            let data = reader.read(v.at + off as u64, size)?;
            let mut streamed = vec![0u8; ssize];
            if ssize > 0 {
                if stream_file.is_none() {
                    stream_file = Some(std::fs::File::open(stream_of(&path))?);
                }
                let f = stream_file.as_mut().unwrap();
                f.seek(SeekFrom::Start(v.stream_at + soff as u64))?;
                f.read_exact(&mut streamed)?;
            }
            clips.insert(clip, (data, streamed));
        }
    }
    Ok(out)
}

/// `game_clips` of a whole bank a mod holds.
fn pieces_clips(pieces: &[Piece], wanted: &BTreeSet<u64>) -> Result<HeldClips> {
    let mut out = HeldClips::new();
    for p in pieces {
        let (ids, clips) = out.entry(p.lang).or_default();
        for r in crate::bank::rows_of(&p.data)? {
            ids.insert(r.clip);
            if wanted.contains(&r.clip) {
                let streamed = r.stream_off.checked_add(r.stream_size).and_then(|end| p.stream.get(r.stream_off..end));
                let streamed = streamed.ok_or_else(|| ObsidianError::BadBank("a clip runs past the end of its stream".into()))?;
                clips.insert(r.clip, (p.data[r.off..r.off + r.size].to_vec(), streamed.to_vec()));
            }
        }
    }
    Ok(out)
}

/// What the overlay deploys: the master with the new ids in it, its flags, and for each source the
/// pieces of Obsidian's bank beside it, by language. In each language of the bank, a replaced clip plays
/// what a rebuild of that bank gave it: the mod's clip in that language, else the bank's own clip there.
fn overlay_made(contents: &Path, o: &Overlay) -> Result<(Vec<u8>, u32, Vec<Vec<Piece>>)> {
    use crate::overlay;
    let (master, flags) = match &o.master_base {
        Some(m) => (m.clone(), 0),
        None => {
            let mut found = None;
            let flags = game_resource(contents, &o.bundle, hash_name("timpani_master"), o.master, |p| {
                if found.is_none() || p.lang == 0 {
                    found = Some(p.data);
                }
                Ok(())
            })?;
            (found.ok_or_else(|| ObsidianError::BadMaster("it is empty".into()))?, flags)
        }
    };
    let mut taken: BTreeSet<u64> = overlay::clip_places(&master)?.into_iter().map(|p| p.1).collect();
    let wanted: BTreeSet<u64> = o.clips.keys().copied().collect();
    let mut held_by = Vec::with_capacity(o.sources.len());
    for s in &o.sources {
        let held = match &s.base {
            Some((_, pieces)) => pieces_clips(pieces, &wanted)?,
            None => game_clips(contents, &s.bundles[0], s.bank, &wanted)?,
        };
        let bank = format!("#{:016x}", s.bank);
        for (lang, clip, owner) in &s.wanted {
            let Some((ids, _)) = held.get(lang) else {
                return Err(ObsidianError::RecipeLanguage { mod_id: owner.clone(), bank, lang: *lang });
            };
            if !ids.contains(clip) {
                return Err(ObsidianError::RecipeClip { mod_id: owner.clone(), bank, clip: format!("#{clip:016x}"), lang: *lang });
            }
        }
        for (ids, _) in held.values() {
            taken.extend(ids);
        }
        held_by.push(held);
    }
    let map = overlay::new_ids(&wanted, &taken);
    let master = overlay::remap(&master, &map)?;
    let mut files: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    let mut read = |p: &Path| -> Result<Vec<u8>> {
        if !files.contains_key(p) {
            files.insert(p.to_path_buf(), std::fs::read(p)?);
        }
        Ok(files[p].clone())
    };
    let mut banks = Vec::with_capacity(o.sources.len());
    for (s, held) in o.sources.iter().zip(&held_by) {
        let mine: BTreeSet<u64> = s.wanted.iter().map(|(_, c, _)| *c).collect();
        let mut heard: BTreeMap<u32, BTreeMap<u64, overlay::ClipBytes>> = BTreeMap::new();
        for (lang, (_, own)) in held {
            for clip in &mine {
                let bytes = match o.clips[clip].get(lang) {
                    Some(file) => {
                        let data = read(&file.data)?;
                        let name = || file.data.file_name().unwrap_or_default().to_string_lossy().into_owned();
                        crate::bank::check_clip(&data).map_err(|why| ObsidianError::RecipeFile { mod_id: file.owner.clone(), file: name(), why })?;
                        let stream = match &file.stream {
                            Some(st) => read(st)?,
                            None => Vec::new(),
                        };
                        (data, stream)
                    }
                    // a language without the clip in this bank: the clip as language 0 of the bank plays it
                    None => match own.get(clip).or_else(|| held.get(&0).and_then(|(_, c)| c.get(clip))) {
                        Some(b) => b.clone(),
                        None => match o.clips[clip].get(&0) {
                            Some(file) => (read(&file.data)?, file.stream.as_deref().map(&mut read).transpose()?.unwrap_or_default()),
                            None => return Err(ObsidianError::BadBank(format!("clip #{clip:016x} has no audio in bank #{:016x}", s.bank))),
                        },
                    },
                };
                heard.entry(*lang).or_default().insert(map[clip], bytes);
            }
        }
        heard.entry(0).or_default();
        let mut pieces = Vec::new();
        for lang in overlay::languages(&heard) {
            let clips: Vec<(u64, &[u8], &[u8])> = heard[&lang].iter().map(|(id, (d, st))| (*id, d.as_slice(), st.as_slice())).collect();
            let (data, stream) = overlay::make_bank(&clips)?;
            pieces.push(Piece { lang, data, stream });
        }
        banks.push(pieces);
    }
    Ok((master, flags, banks))
}

/// The bundle that holds the game's master, and the master's name: where the overlay goes. None for a
/// game without one, or with `SOUND_REBUILD` set: each bank with new clips is then rebuilt whole.
fn game_master(contents: &Path) -> Option<(String, u64)> {
    if std::env::var_os(SOUND_REBUILD).is_some() {
        return None;
    }
    let master = hash_name("timpani_master");
    let found = holders(contents);
    let mut all: Vec<(String, u64)> = found.iter().filter(|((t, _), _)| *t == master).flat_map(|((_, n), b)| b.iter().map(move |b| (b.clone(), *n))).collect();
    all.sort();
    all.into_iter().next()
}

/// When each file under a folder last changed and how big the file is, from the folder listings: on Windows
/// a listing carries both, where asking file by file costs a call each.
fn file_stamps(dir: &Path) -> std::collections::HashMap<PathBuf, Stamp> {
    let mut out = std::collections::HashMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                todo.push(e.path());
            } else if let Ok(t) = meta.modified() {
                out.insert(e.path(), (t, meta.len()));
            }
        }
    }
    out
}

/// The resource that wins one (bundle, type, name), and the earlier owners it replaced with the digests of their data.
type Slot = (Planned, Vec<(String, [u8; 32])>);

/// The game's generated tables a mod has to change to add a unit. Mods that do can't take turns: each needs
/// its entries there, so these merge instead of the last one winning.
const MERGED_TABLES: [&str; 4] = ["lua/generated/settings_paths", "lua/generated/units_with_settings", "lua/generated/unit_paths", "lua/generated/unit_path_to_network_object"];
const MERGE_SCRIPT: &str = include_str!("../data/merge_tables.lua");

pub(crate) fn merged_table(type_hash: u64, name_hash: u64) -> Option<&'static str> {
    (type_hash == hash_name("lua")).then(|| MERGED_TABLES.iter().find(|n| hash_name(n) == name_hash).copied()).flatten()
}

/// One table from several mods' versions: each as a script of its own, and a script in the table's place that
/// asks for them and puts them together when the game asks for the table.
fn merge_table(table: &str, versions: &[Planned], with_data: bool) -> Result<Vec<Planned>> {
    use sha2::{Digest, Sha256};
    let last = versions.last().expect("a merge has versions");
    let mut h = Sha256::new();
    h.update(b"merged table");
    let mut out = Vec::with_capacity(versions.len() + 1);
    let mut names = Vec::with_capacity(versions.len());
    for v in versions {
        h.update(v.digest);
        let name = format!("obsidian/merged/{}/{table}", v.owner);
        out.push(Planned { name_hash: hash_name(&name), ..v.clone() });
        names.push(format!("{name:?}"));
    }
    let data = if with_data {
        let source = format!("local NAMES = {{ {} }}\n{MERGE_SCRIPT}", names.join(", "));
        wrap_resource(&compile(source.as_bytes(), &format!("obsidian/merged/{table}"))?)
    } else {
        Vec::new()
    };
    out.push(Planned { data, digest: h.finalize().into(), ..last.clone() });
    Ok(out)
}

fn compose_with(lib: &Library, profile: &Profile, with_data: bool, contents: Option<&Path>, known: Option<&BTreeMap<String, Manifest>>, on_mod: Option<OnMod<'_>>) -> Result<Composition> {
    let mut slots: BTreeMap<(String, u64, u64), Slot> = BTreeMap::new();
    // Every mod's version of a table that merges, in load order, with one entry for the same bytes.
    let mut table_versions: BTreeMap<(String, u64), (&str, Vec<Planned>)> = BTreeMap::new();
    let mut mods = Vec::new();
    let mut unplaced = Vec::new();
    let reserved = (hash_name("lua"), hash_name(crate::loader::ENTRY));
    let mut worked_out = false;
    if !with_data || contents.is_some() {
        load_digests(lib.data_dir());
    }
    let total = profile.mods.iter().filter(|m| m.enabled).count();
    for (index, entry) in profile.mods.iter().filter(|m| m.enabled).enumerate() {
        let dir = lib.mod_dir(&entry.id);
        if !dir.join(crate::manifest::FILE).is_file() {
            return Err(ObsidianError::ModMissing(entry.id.clone()));
        }
        // `Library::broken` reports a mod that can't be read, and it stays out until it's fixed.
        let m = match known.and_then(|k| k.get(&entry.id)) {
            Some(m) => m.clone(),
            None => match load_manifest(&dir) {
                Ok(m) => m,
                Err(_) => continue,
            },
        };
        if let Some(tell) = on_mod {
            tell(index, total, &m);
        }
        let values = effective_values(&m.option_specs()?, profile.options.get(&m.id));
        if m.resources.iter().any(|r| (r.type_hash, r.name_hash) == reserved) {
            return Err(ObsidianError::ReservedResource { mod_id: m.id.clone(), name: crate::loader::ENTRY.into() });
        }
        let chosen: Vec<_> = m.resources.iter().filter(|r| when_matches(&r.when, &values)).collect();
        let stamps = if with_data { Default::default() } else { file_stamps(&dir) };
        for r in &chosen {
            if let Payload::Clips(list) = &r.payload {
                let Some(contents) = contents else {
                    if with_data {
                        return Err(ObsidianError::GameNotFound(String::new()));
                    }
                    unplaced.push(m.id.clone());
                    continue;
                };
                let found = holders(contents);
                let Some(bundles) = found.get(&(r.type_hash, r.name_hash)) else {
                    if with_data {
                        return Err(ObsidianError::RecipeBank { mod_id: m.id.clone(), bank: format!("#{:016x}", r.name_hash) });
                    }
                    unplaced.push(m.id.clone());
                    continue;
                };
                let mut clips = Vec::with_capacity(list.len());
                for c in list {
                    use sha2::{Digest, Sha256};
                    let data = dir.join(&c.data);
                    let stream = c.stream.as_ref().map(|s| dir.join(s));
                    let mut h = Sha256::new();
                    h.update(file_digest(&data, &stamps, &mut worked_out)?);
                    if let Some(s) = &stream {
                        h.update(file_digest(s, &stamps, &mut worked_out)?);
                    }
                    clips.push(((c.lang, c.id()), ClipFile { owner: m.id.clone(), data, stream, digest: h.finalize().into() }));
                }
                for bundle in bundles {
                    let key = (bundle.clone(), r.type_hash, r.name_hash);
                    let (mut planned, earlier) = match slots.remove(&key) {
                        Some((old, earlier)) if old.recipe.is_some() => (old, earlier),
                        Some((old, earlier)) => {
                            let pieces = if !old.pieces.is_empty() {
                                old.pieces
                            } else {
                                vec![Piece { lang: 0, data: old.data, stream: Vec::new() }]
                            };
                            let base = Base { owner: old.owner, digest: old.digest, pieces: if with_data { pieces } else { Vec::new() } };
                            let recipe = Recipe { base: Some(base), first: bundles[0].clone(), ..Default::default() };
                            (Planned { flags: old.flags, data: Vec::new(), pieces: Vec::new(), recipe: Some(recipe), ..Planned::new(r.type_hash, r.name_hash, 0, Vec::new(), m.id.clone()) }, earlier)
                        }
                        None => {
                            let recipe = Recipe { first: bundles[0].clone(), ..Default::default() };
                            (Planned { recipe: Some(recipe), ..Planned::new(r.type_hash, r.name_hash, 0, Vec::new(), m.id.clone()) }, Vec::new())
                        }
                    };
                    planned.owner = m.id.clone();
                    let recipe = planned.recipe.as_mut().unwrap();
                    for (place, file) in &clips {
                        if let Some(old) = recipe.clips.insert(*place, file.clone()) {
                            if old.digest != file.digest && old.owner != file.owner {
                                recipe.lost.push((old.owner, file.owner.clone()));
                            }
                        }
                    }
                    slots.insert(key, (planned, earlier));
                }
                continue;
            }
            let bundle = r.bundle.clone().ok_or_else(|| ObsidianError::BundleRequired { mod_id: m.id.clone() })?;
            let path = dir.join(r.data_path());
            let game_data = || -> Result<Vec<u8>> {
                let bytes = std::fs::read(&path)?;
                Ok(match &r.payload {
                    Payload::Data(_) | Payload::Variants(_) | Payload::Clips(_) => bytes,
                    Payload::Bytecode(_) => {
                        check_bytecode(&bytes)?;
                        wrap_resource(&bytes)
                    }
                    Payload::Source(p) => {
                        let chunk = format!("{}/{}", m.id, p.to_string_lossy().replace('\\', "/"));
                        wrap_resource(&compile(&bytes, &chunk)?)
                    }
                })
            };
            let game_pieces = |variants: &[crate::manifest::VariantSpec]| -> Result<Vec<Piece>> {
                variants
                    .iter()
                    .map(|v| {
                        let stream = match &v.stream {
                            Some(s) => std::fs::read(dir.join(s))?,
                            None => Vec::new(),
                        };
                        Ok(Piece { lang: v.lang, data: std::fs::read(dir.join(&v.data))?, stream })
                    })
                    .collect()
            };
            let planned = match (&r.payload, with_data) {
                (Payload::Variants(v), true) => Planned::with_pieces(r.type_hash, r.name_hash, r.flags, game_pieces(v)?, m.id.clone()),
                (_, true) => Planned::new(r.type_hash, r.name_hash, r.flags, game_data()?, m.id.clone()),
                (payload, false) => {
                    // A resource in pieces is stamped by its newest file and their total size.
                    let mut stamp: Option<Stamp> = None;
                    for p in payload.paths() {
                        let p = dir.join(p);
                        let s = match stamps.get(&p) {
                            Some(s) => *s,
                            None => {
                                let meta = std::fs::metadata(&p)?;
                                (meta.modified()?, meta.len())
                            }
                        };
                        stamp = Some(stamp.map_or(s, |(t, n)| (t.max(s.0), n + s.1)));
                    }
                    let stamp = stamp.expect("a payload names at least one file");
                    let cached = digests().lock().unwrap().get(&path).filter(|(s, _)| *s == stamp).map(|(_, d)| *d);
                    let digest = match cached {
                        Some(d) => d,
                        None => {
                            let d = match payload {
                                Payload::Variants(v) => pieces_digest(&game_pieces(v)?),
                                _ => sha256(&game_data()?),
                            };
                            digests().lock().unwrap().insert(path.clone(), (stamp, d));
                            worked_out = true;
                            d
                        }
                    };
                    Planned { type_hash: r.type_hash, name_hash: r.name_hash, flags: r.flags, data: Vec::new(), digest, owner: m.id.clone(), pieces: Vec::new(), recipe: None }
                }
            };
            if let Some(table) = merged_table(r.type_hash, r.name_hash) {
                let (_, versions) = table_versions.entry((bundle.clone(), r.name_hash)).or_insert((table, Vec::new()));
                if !versions.iter().any(|v| v.digest == planned.digest) {
                    versions.push(planned.clone());
                }
            }
            let key = (bundle, r.type_hash, r.name_hash);
            let earlier = match slots.remove(&key) {
                Some((old, mut earlier)) => {
                    match &old.recipe {
                        // A whole bank replaces the clips below it, and whatever they went on.
                        Some(recipe) => {
                            earlier.extend(recipe.base.iter().map(|b| (b.owner.clone(), b.digest)));
                            for file in recipe.clips.values() {
                                if !earlier.iter().any(|(owner, _)| *owner == file.owner) {
                                    earlier.push((file.owner.clone(), [0; 32]));
                                }
                            }
                        }
                        None => earlier.push((old.owner, old.digest)),
                    }
                    earlier
                }
                None => Vec::new(),
            };
            slots.insert(key, (planned, earlier));
        }
        let resources = chosen.len();
        mods.push(ComposedMod { manifest: m, dir, values, resources });
    }
    if with_data {
        for c in &mods {
            if let Some(id) = c.manifest.requires.iter().find(|id| !mods.iter().any(|m| m.manifest.id == **id)) {
                let (mod_id, requires) = (c.manifest.id.clone(), id.clone());
                return Err(if lib.mod_dir(id).is_dir() { ObsidianError::MissingRequirement { mod_id, requires } } else { ObsidianError::RequirementNotInstalled { mod_id, requires } });
            }
        }
    }
    let mut out = Composition { mods, unplaced, sandbox: crate::saves::active_sandbox(lib.data_dir()).is_some(), ..Default::default() };
    // The game's master, which takes the clips of every recipe with Obsidian's bank beside it (overlay.rs).
    let master = if slots.values().any(|(p, _)| p.recipe.is_some()) { contents.and_then(game_master) } else { None };
    let mut overlaid = Vec::new();
    for ((bundle, type_hash, name_hash), (mut planned, earlier)) in slots {
        if let Some((table, versions)) = table_versions.get(&(bundle.clone(), name_hash)).filter(|(_, v)| v.len() > 1 && planned.recipe.is_none()) {
            out.bundles.entry(bundle).or_default().extend(merge_table(table, versions, with_data)?);
            continue;
        }
        if let Some(recipe) = &planned.recipe {
            let base = match &recipe.base {
                Some(b) => Ok(b.digest),
                None => game_digest(contents.expect("a recipe has a game"), &recipe.first, name_hash),
            };
            let base = match base {
                Ok(d) => d,
                Err(e) if with_data => return Err(e),
                Err(_) => [0; 32],
            };
            planned.digest = recipe_digest(&base, recipe);
            // A bank in several bundles is one conflict.
            if recipe.first == bundle {
                let mut winners: Vec<&String> = recipe.lost.iter().map(|(_, w)| w).collect();
                winners.sort();
                winners.dedup();
                for winner in winners {
                    let mut losers: Vec<String> = recipe.lost.iter().filter(|(_, w)| w == winner).map(|(l, _)| l.clone()).collect();
                    losers.sort();
                    losers.dedup();
                    out.conflicts.push(Conflict { bundle: bundle.clone(), type_hash, name_hash, winner: winner.clone(), losers });
                }
            }
        }
        // A mod carrying the same data as the winner loses nothing.
        let losers: Vec<String> = earlier.into_iter().filter(|(_, digest)| *digest != planned.digest).map(|(owner, _)| owner).collect();
        if !losers.is_empty() {
            out.conflicts.push(Conflict { bundle: bundle.clone(), type_hash, name_hash, winner: planned.owner.clone(), losers });
        }
        if master.is_some() {
            if let Some(recipe) = planned.recipe.take() {
                // The clips go into the overlay. The bank stays as the game has it, or as a whole bank
                // of a mod below the clips has it.
                if let Some(b) = &recipe.base {
                    let mut whole = Planned::with_pieces(type_hash, name_hash, planned.flags, b.pieces.clone(), b.owner.clone());
                    whole.digest = b.digest;
                    out.bundles.entry(bundle.clone()).or_default().push(whole);
                }
                overlaid.push((bundle, name_hash, recipe, planned.digest));
                continue;
            }
        }
        out.bundles.entry(bundle).or_default().push(planned);
    }
    if let Some((bundle, name)) = master.filter(|_| !overlaid.is_empty()) {
        add_overlay(&mut out, contents.expect("a recipe has a game"), bundle, name, overlaid, with_data)?;
    }
    if worked_out {
        save_digests(lib.data_dir());
    }
    Ok(out)
}

/// Puts the clips of every recipe into the master, in the master's bundle on top of a whole master a mod
/// has there, and into a bank of Obsidian's beside each bank they replace, in every bundle that holds that
/// bank. `overlaid` is each recipe with its bundle, its bank and its digest, in the order of the slots.
fn add_overlay(out: &mut Composition, contents: &Path, bundle: String, name: u64, overlaid: Vec<(String, u64, Recipe, [u8; 32])>, with_data: bool) -> Result<()> {
    use sha2::{Digest, Sha256};
    let rank = |owner: &str| out.mods.iter().position(|m| m.manifest.id == owner);
    let mut clips: BTreeMap<u64, BTreeMap<u32, ClipFile>> = BTreeMap::new();
    let mut sources: Vec<Source> = Vec::new();
    let mut h = Sha256::new();
    h.update(b"timpani overlay");
    for (at, bank, recipe, digest) in overlaid {
        h.update(digest);
        for ((lang, clip), file) in &recipe.clips {
            let slot = clips.entry(*clip).or_default();
            if slot.get(lang).is_none_or(|old| rank(&old.owner) <= rank(&file.owner)) {
                slot.insert(*lang, file.clone());
            }
        }
        let wanted: Vec<(u32, u64, String)> = recipe.clips.iter().map(|((l, c), f)| (*l, *c, f.owner.clone())).collect();
        let base = recipe.base.map(|b| (b.digest, b.pieces));
        let key = base.as_ref().map(|(d, _)| *d);
        match sources.iter_mut().find(|s| s.bank == bank && s.base.as_ref().map(|(d, _)| *d) == key) {
            Some(s) => {
                let more: Vec<_> = wanted.into_iter().filter(|w| !s.wanted.contains(w)).collect();
                s.wanted.extend(more);
                if !s.bundles.contains(&at) {
                    s.bundles.push(at);
                }
            }
            None => sources.push(Source { bank, bundles: vec![at], base, wanted }),
        }
    }
    let overlay_digest: [u8; 32] = h.finalize().into();
    let list = out.bundles.entry(bundle.clone()).or_default();
    let master_type = hash_name("timpani_master");
    let base = list.iter().position(|p| (p.type_hash, p.name_hash) == (master_type, name)).map(|i| list.remove(i));
    let base_digest = match &base {
        Some(p) => Ok(p.digest),
        None => game_digest(contents, &bundle, name),
    };
    let base_digest = match base_digest {
        Ok(d) => d,
        Err(e) if with_data => return Err(e),
        Err(_) => [0; 32],
    };
    let mut h = Sha256::new();
    h.update(b"timpani master");
    h.update(base_digest);
    h.update(overlay_digest);
    let master_digest: [u8; 32] = h.finalize().into();
    let owner = clips.values().flat_map(|l| l.values()).max_by_key(|f| rank(&f.owner)).map(|f| f.owner.clone()).unwrap_or_default();
    let flags = base.as_ref().map_or(0, |p| p.flags);
    let master_base = base.filter(|_| with_data).map(|p| if p.pieces.is_empty() { p.data } else { p.pieces.into_iter().min_by_key(|x| x.lang).unwrap().data });
    // where each bank of new clips goes, and under which name: one per bank of the game, and one more
    // where a whole bank of a mod sits below the clips
    let placed: Vec<(Vec<String>, u64, [u8; 32])> = sources
        .iter()
        .map(|s| {
            let name = match &s.base {
                Some((d, _)) => format!("{}/{:016x}/{:02x}{:02x}{:02x}{:02x}", crate::overlay::BANK_NAME, s.bank, d[0], d[1], d[2], d[3]),
                None => format!("{}/{:016x}", crate::overlay::BANK_NAME, s.bank),
            };
            let mut h = Sha256::new();
            h.update(overlay_digest);
            h.update(name.as_bytes());
            (s.bundles.clone(), hash_name(&name), h.finalize().into())
        })
        .collect();
    let overlay = std::sync::Arc::new(Overlay { bundle: bundle.clone(), master: name, master_base, clips, sources });
    let recipe = |part| Some(Recipe { overlay: Some(part), ..Default::default() });
    let mut m = Planned { recipe: recipe(Part::Master(overlay.clone())), ..Planned::new(master_type, name, flags, Vec::new(), owner.clone()) };
    m.digest = master_digest;
    out.bundles.entry(bundle).or_default().push(m);
    for (i, (bundles, bank_name, digest)) in placed.into_iter().enumerate() {
        for at in bundles {
            let mut b = Planned { recipe: recipe(Part::Bank(overlay.clone(), i)), ..Planned::new(hash_name("timpani_bank"), bank_name, 0, Vec::new(), owner.clone()) };
            b.digest = digest;
            out.bundles.entry(at).or_default().push(b);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::ProfileEntry;
    use crate::testutil::{put, spaced_tempdir};

    const VANILLA_BODY_HEX: &str = "1b4c4a0102450200020003000434000000250102003a0101004700010018323032302d31322d32322030393a33333a31341b4255494c445f444154455f494e464f524d4154494f4e075f4700";

    /// Installs a mod whose one resource is `lua/#..01` in bundle `..aa`, from `kind` = data/bytecode/source.
    fn add_mod(t: &std::path::Path, lib: &Library, id: &str, kind: &str, bytes: &[u8]) {
        let d = t.join(format!("stage-{id}"));
        let json = format!(
            r##"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},
               "resources":[{{"type":"lua","name":"#0000000000000001","bundle":"00000000000000aa","{kind}":"r.bin"}}]}}"##
        );
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "r.bin", bytes);
        lib.install(&d).unwrap();
    }

    fn profile(ids: &[(&str, bool)]) -> Profile {
        Profile { mods: ids.iter().map(|(id, on)| ProfileEntry { id: id.to_string(), enabled: *on, folder: None }).collect(), ..Default::default() }
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn later_mod_wins_and_conflict_is_recorded() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_mod(t.path(), &lib, "first", "data", b"one");
        add_mod(t.path(), &lib, "second", "data", b"two");
        let c = compose(&lib, &profile(&[("first", true), ("second", true)])).unwrap();
        assert_eq!(c.bundles["00000000000000aa"][0].data, b"two");
        assert_eq!(c.conflicts.len(), 1);
        assert_eq!((c.conflicts[0].winner.as_str(), c.conflicts[0].losers.clone()), ("second", vec!["first".to_string()]));
    }

    #[test]
    fn the_same_data_in_two_mods_is_no_conflict() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_mod(t.path(), &lib, "first", "data", b"same");
        add_mod(t.path(), &lib, "second", "data", b"same");
        add_mod(t.path(), &lib, "third", "data", b"other");
        let both = profile(&[("first", true), ("second", true)]);
        assert!(compose(&lib, &both).unwrap().conflicts.is_empty());
        assert!(plan(&lib, &both).unwrap().conflicts.is_empty());
        let c = compose(&lib, &profile(&[("first", true), ("second", true), ("third", true)])).unwrap();
        assert_eq!(c.conflicts.len(), 1);
        assert_eq!(c.conflicts[0].losers, ["first", "second"], "both lose to data that differs");
        let c = compose(&lib, &profile(&[("third", true), ("first", true), ("second", true)])).unwrap();
        assert_eq!(c.conflicts[0].losers, ["third"], "the mod that matches the winner stays out of the list");
    }

    #[test]
    fn disabled_mods_contribute_nothing() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_mod(t.path(), &lib, "first", "data", b"one");
        assert!(compose(&lib, &profile(&[("first", false)])).unwrap().bundles.is_empty());
    }

    #[test]
    fn bytecode_is_wrapped_and_checked() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_mod(t.path(), &lib, "bc", "bytecode", &hex(VANILLA_BODY_HEX));
        add_mod(t.path(), &lib, "bad", "bytecode", &[0x1b, 0x4c, 0x4a, 0x02, 0x02]);
        let c = compose(&lib, &profile(&[("bc", true)])).unwrap();
        assert_eq!(&c.bundles["00000000000000aa"][0].data[..8], &[76, 0, 0, 0, 2, 0, 0, 0]);
        assert!(matches!(compose(&lib, &profile(&[("bad", true)])), Err(ObsidianError::InvalidBytecode(_))));
    }

    fn add_table_mod(t: &std::path::Path, lib: &Library, id: &str, table: &str, bytes: &[u8]) {
        let d = t.join(format!("stage-{id}"));
        let json = format!(
            r##"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},
               "resources":[{{"type":"lua","name":"{table}","bundle":"00000000000000aa","source":"r.bin"}}]}}"##
        );
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "r.bin", bytes);
        lib.install(&d).unwrap();
    }

    #[test]
    fn mods_changing_a_generated_table_merge_instead_of_conflicting() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let table = "lua/generated/settings_paths";
        add_table_mod(t.path(), &lib, "one", table, b"return { 'game', 'one' }");
        add_table_mod(t.path(), &lib, "two", table, b"return { 'game', 'two' }");
        add_table_mod(t.path(), &lib, "same", table, b"return { 'game', 'two' }");
        let p = profile(&[("one", true), ("two", true), ("same", true)]);
        let c = compose(&lib, &p).unwrap();
        assert!(c.conflicts.is_empty(), "{:?}", c.conflicts);
        let list = &c.bundles["00000000000000aa"];
        assert_eq!(list.len(), 3, "the same bytes need one script");
        let lua = hash_name("lua");
        let name = |id: &str| hash_name(&format!("obsidian/merged/{id}/{table}"));
        assert!(list.iter().any(|r| r.type_hash == lua && r.name_hash == name("one")));
        assert!(list.iter().any(|r| r.type_hash == lua && r.name_hash == name("two")));
        let merged = list.iter().find(|r| r.name_hash == hash_name(table)).unwrap();
        crate::lua::check_bytecode(crate::lua::unwrap_resource(&merged.data).unwrap()).unwrap();
        // A plan with no data says the same, so a deploy isn't called for after one.
        let planned = plan(&lib, &p).unwrap();
        let again = planned.bundles["00000000000000aa"].iter().find(|r| r.name_hash == hash_name(table)).unwrap();
        assert_eq!(again.digest, merged.digest);
    }

    #[test]
    fn one_mod_on_a_generated_table_stays_as_it_is_and_other_scripts_still_conflict() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_table_mod(t.path(), &lib, "one", "lua/generated/unit_paths", b"return { 'game' }");
        let c = compose(&lib, &profile(&[("one", true)])).unwrap();
        assert_eq!(c.bundles["00000000000000aa"].len(), 1);
        add_table_mod(t.path(), &lib, "a", "lua/settings/perk_settings", b"return 1");
        add_table_mod(t.path(), &lib, "b", "lua/settings/perk_settings", b"return 2");
        let c = compose(&lib, &profile(&[("a", true), ("b", true)])).unwrap();
        assert_eq!(c.conflicts.len(), 1);
    }

    #[test]
    fn source_resource_is_compiled_and_wrapped() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_mod(t.path(), &lib, "src", "source", b"return 1");
        let c = compose(&lib, &profile(&[("src", true)])).unwrap();
        let res = &c.bundles["00000000000000aa"][0].data;
        crate::lua::check_bytecode(crate::lua::unwrap_resource(res).unwrap()).unwrap();
    }

    #[test]
    fn source_error_names_the_mod_and_file() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add_mod(t.path(), &lib, "src", "source", b"return (");
        match compose(&lib, &profile(&[("src", true)])) {
            Err(ObsidianError::CompileFailed { chunk, .. }) => assert_eq!(chunk, "src/r.bin"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn deploy_options_filter_resources() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage");
        put(&d, "obsidian.json", br##"{"obsidian":1,"id":"opt","version":"1.0.0","name":{"en":"o"},
            "options":[{"id":"sky","type":"choice","default":"day","choices":["day","night"]}],
            "resources":[
              {"type":"lua","name":"#0000000000000001","bundle":"00000000000000aa","data":"day.bin","when":{"sky":"day"}},
              {"type":"lua","name":"#0000000000000001","bundle":"00000000000000aa","data":"night.bin","when":{"sky":"night"}}]}"##);
        put(&d, "day.bin", b"day");
        put(&d, "night.bin", b"night");
        lib.install(&d).unwrap();
        let mut p = profile(&[("opt", true)]);
        let c = compose(&lib, &p).unwrap();
        assert_eq!(c.bundles["00000000000000aa"].iter().map(|r| r.data.as_slice()).collect::<Vec<_>>(), vec![b"day".as_slice()]);
        assert!(c.conflicts.is_empty(), "an option switched off never reports a conflict");
        p.options.insert("opt".into(), [("sky".to_string(), serde_json::json!("night"))].into());
        assert_eq!(compose(&lib, &p).unwrap().bundles["00000000000000aa"][0].data, b"night");
    }

    #[test]
    fn reserved_resource_is_refused() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage");
        put(&d, "obsidian.json", br##"{"obsidian":1,"id":"grab","version":"1.0.0","name":{"en":"g"},
            "resources":[{"type":"lua","name":"foundation/lua/build_date_information","bundle":"9cf71054ea26dddf","data":"r.bin"}]}"##);
        put(&d, "r.bin", b"x");
        lib.install(&d).unwrap();
        let e = compose(&lib, &profile(&[("grab", true)])).unwrap_err();
        assert!(matches!(e, ObsidianError::ReservedResource { ref mod_id, .. } if mod_id == "grab"), "{e}");
    }

    #[test]
    fn missing_mod_is_an_error() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        assert!(matches!(compose(&lib, &profile(&[("ghost", true)])), Err(ObsidianError::ModMissing(_))));
    }
}
