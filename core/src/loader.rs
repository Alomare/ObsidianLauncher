//! Obsidian Loader as patch resources: the loader modules, every mod entry and the deploy table,
//! all compiled for the boot bundle, plus the settings file the loader reads at launch.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::compose::{ComposedMod, Composition, Planned};
use crate::hash::hash_name;
use crate::library::{Library, Profile};
use crate::lua::{check_bytecode, unwrap_resource, wrap_resource};
use crate::luajit::compile_all;
use crate::manifest::{Manifest, Scope, load_manifest};
use crate::modset::{SessionMod, every_hash, every_pool_tag, expansion_pool_tag, modset_hash, open_pool_tag, sandbox_pool_tag};
use crate::options::{effective_values, value_text};
use crate::settings::Settings;
use crate::{ObsidianError, Result};

/// The boot bundle, which holds `foundation/lua/build_date_information`.
pub const BUNDLE: &str = "9cf71054ea26dddf";
/// The folder of the loader's own fonts in the game.
const FONT_BASE: &str = "obsidian/fonts";
/// The vanilla chunk the game requires at boot; the loader takes the chunk's place.
pub const ENTRY: &str = "foundation/lua/build_date_information";
/// The start of `BUILD_REVISION_INFORMATION` on the supported build.
pub const REVISION: &str = "parent: 32148:3bd83d77c8ae";
/// The languages of the game, by the name a catalog file has in a mod's `locales` folder. The loader
/// maps the game's own codes to these.
const LANGUAGES: [&str; 17] = ["en", "fr", "es", "pt-BR", "de", "it", "ru", "pl", "ko", "ja", "zh-CN", "zh-TW", "sv", "da", "nl", "fi", "no"];

/// `rel` in the app's resources, beside the exe, then in this workspace. Debug builds look in the
/// workspace first: the copy beside a dev exe is only refreshed when the Rust side rebuilds, so it
/// goes stale after a change to a starter mod or the loader alone.
fn candidates(rel: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = crate::paths::resource_dir().map(|d| d.join(rel)).into_iter().collect();
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        out.push(dir.join(rel));
    }
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    if cfg!(debug_assertions) {
        out.insert(0, workspace);
    } else {
        out.push(workspace);
    }
    out
}

/// The folder of the starter pack (`bundled/` beside the exe, then in this workspace), if there is one.
pub fn find_bundled_dir() -> Option<PathBuf> {
    candidates("bundled").into_iter().find(|d| d.is_dir())
}

/// The loader's baked fonts, `loader/fonts` beside `loader/src`: a type, a resource name and a file each.
fn own_fonts(loader_dir: &Path) -> Vec<(&'static str, String, PathBuf)> {
    let Ok(dir) = std::fs::read_dir(loader_dir.join("..").join("fonts")) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = dir.filter_map(|e| e.ok().map(|e| e.path())).collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let kind = match p.extension()?.to_str()? {
                "font" => "font",
                "texture" => "texture",
                "material" => "material",
                _ => return None,
            };
            let stem = p.file_stem()?.to_str()?.to_string();
            Some((kind, format!("{FONT_BASE}/{stem}"), p))
        })
        .collect()
}

/// What a gui may list and a text may name: fonts and materials this deploy puts in the boot bundle,
/// which is loaded before anything runs and never unloads. Anything else is assumed fatal in the engine.
fn deployed_fonts(comp: &Composition) -> (Vec<String>, Vec<String>) {
    let planned = comp.bundles.get(BUNDLE).map_or(&[][..], Vec::as_slice);
    let (mut files, mut paths) = (Vec::new(), Vec::new());
    for r in comp.mods.iter().flat_map(|c| &c.manifest.resources) {
        if r.bundle.as_deref() != Some(BUNDLE) || !planned.iter().any(|p| p.type_hash == r.type_hash && p.name_hash == r.name_hash) {
            continue;
        }
        // By the hash: a manifest can write the type as "#eac0b497876adedf".
        if r.type_hash == hash_name("material") {
            files.push(r.name_text.clone());
        } else if r.type_hash == hash_name("font") {
            paths.push(r.name_text.clone());
        }
    }
    (files, paths)
}

/// The deploy table, with the loader's own fonts added to what the mods bring.
fn full_table(loader_dir: &Path, comp: &Composition) -> Result<Value> {
    let texts = read_texts(&loader_dir.join("..").join("..").join("locales"))?;
    let mut table = deploy_table(comp, texts)?;
    for (kind, name, _) in own_fonts(loader_dir) {
        let key = match kind {
            "material" => "font_files",
            "font" => "font_paths",
            _ => continue,
        };
        if let Some(list) = table[key].as_array_mut() {
            list.push(json!(name));
        }
    }
    Ok(table)
}

/// `loader/src` beside the exe, then the one in this workspace.
pub fn find_loader_dir() -> Result<PathBuf> {
    let candidates = candidates("loader/src");
    candidates.iter().find(|d| d.join("boot.lua").is_file()).cloned().ok_or_else(|| {
        ObsidianError::LoaderMissing(candidates.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", "))
    })
}

/// The loader goes into the game whenever a mod or a guardrail of the account is on. Blocking other
/// players' session mods alone needs no loader: a game without it runs none, and the others treat
/// it as a vanilla player.
pub fn needed(comp: &Composition, profile: &Profile) -> bool {
    !comp.mods.is_empty() || profile.block_achievements || profile.freeze_progress || comp.sandbox
}

/// The folder of a mod's resources. Ids are already limited to `[a-z0-9._-]`.
fn mod_root(id: &str) -> String {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    format!("obsidian/mods/{safe}")
}

/// The resource name a mod entry runs from.
pub fn entry_name(id: &str) -> String {
    format!("{}/main", mod_root(id))
}

/// The resource name of a script in `modules`: its path in the mod, without `.lua`.
pub fn module_name(id: &str, path: &Path) -> String {
    let parts: Vec<String> = path.with_extension("").components().map(|c| c.as_os_str().to_string_lossy().into_owned()).filter(|c| c != ".").collect();
    format!("{}/{}", mod_root(id), parts.join("/"))
}

/// The owner of what the loader itself puts in the game: its scripts, fonts and the deploy table.
pub const LOADER_OWNER: &str = "obsidian";

fn lua_resource(name: &str, bytecode: &[u8], owner: &str) -> Planned {
    Planned::new(hash_name("lua"), hash_name(name), 0, wrap_resource(bytecode), owner.to_string())
}

/// One script of a loader build: bytecode already, or source for the compiler with its chunk name.
enum Script {
    Bytecode(Vec<u8>),
    Source(Vec<u8>, String),
}

/// An entry file is bytecode when the file starts with the LuaJIT header or the resource envelope,
/// and source otherwise.
fn entry_script(bytes: Vec<u8>, chunk: &str) -> Result<Script> {
    if bytes.starts_with(&[0x1b, 0x4c, 0x4a]) {
        check_bytecode(&bytes)?;
        return Ok(Script::Bytecode(bytes));
    }
    if let Ok(inner) = unwrap_resource(&bytes) {
        check_bytecode(inner)?;
        return Ok(Script::Bytecode(inner.to_vec()));
    }
    Ok(Script::Source(bytes, chunk.to_string()))
}

/// Every `<lang>.json` found in `dir`, as `{ lang: { key: text } }`.
fn read_texts(dir: &Path) -> Result<Value> {
    let mut out = Map::new();
    for lang in LANGUAGES {
        let path = dir.join(format!("{lang}.json"));
        if !path.is_file() {
            continue;
        }
        let mut v: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)
            .map_err(|e| ObsidianError::BadManifest { field: path.display().to_string(), reason: e.to_string() })?;
        // The shared catalog also holds the app's texts, which the game never shows.
        if let Value::Object(m) = &mut v {
            m.retain(|k, _| !k.starts_with("app."));
        }
        out.insert(lang.to_string(), v);
    }
    Ok(Value::Object(out))
}

/// What the loader gets for a scope, once libraries have taken one: an unspecified mod runs as a
/// client mod.
fn scope_text(s: &Scope) -> &'static str {
    match s {
        Scope::Session => "session",
        Scope::Expansion => "expansion",
        _ => "client",
    }
}

/// The ids of the libraries `from` requires, directly or through another library.
fn libraries_of<'a>(from: &'a ComposedMod, mods: &'a [ComposedMod]) -> BTreeSet<&'a str> {
    let mut found = BTreeSet::new();
    let mut waiting = vec![from];
    while let Some(c) = waiting.pop() {
        for id in &c.manifest.requires {
            if let Some(required) = mods.iter().find(|m| m.manifest.id == *id && m.manifest.scope == Scope::Library) {
                if found.insert(required.manifest.id.as_str()) {
                    waiting.push(required);
                }
            }
        }
    }
    found
}

/// The ids of the libraries that run as session mods: those an enabled session mod requires,
/// directly or through another library. A mod with a session secondary scope on counts, an expansion
/// too, as that code may call guarded functions. Every other library runs as a client mod, or as part of the expansions
/// that require it.
fn session_libraries(mods: &[ComposedMod]) -> BTreeSet<&str> {
    mods.iter()
        .filter(|c| c.manifest.scope_with(&c.values) == Scope::Session || c.manifest.session_outside_content(&c.values))
        .flat_map(|c| libraries_of(c, mods))
        .collect()
}

/// Each library an enabled expansion requires, directly or through another library, with the ids
/// of those expansions: such a library gives them full use while their content runs.
fn expansion_libraries(mods: &[ComposedMod]) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut out: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for c in mods.iter().filter(|c| c.manifest.scope == Scope::Expansion) {
        for lib in libraries_of(c, mods) {
            out.entry(lib).or_default().insert(c.manifest.id.as_str());
        }
    }
    out
}

/// Load order, with each mod moved just far enough to start after the mods it requires.
fn start_order(mods: &[ComposedMod]) -> Vec<&ComposedMod> {
    fn visit<'a>(i: usize, mods: &'a [ComposedMod], seen: &mut [bool], out: &mut Vec<&'a ComposedMod>) {
        if std::mem::replace(&mut seen[i], true) {
            return;
        }
        for id in &mods[i].manifest.requires {
            if let Some(j) = mods.iter().position(|m| m.manifest.id == *id) {
                visit(j, mods, seen, out);
            }
        }
        out.push(&mods[i]);
    }
    let (mut seen, mut out) = (vec![false; mods.len()], Vec::with_capacity(mods.len()));
    for i in 0..mods.len() {
        visit(i, mods, &mut seen, &mut out);
    }
    out
}

/// The deploy table as JSON: version, revision, pool hashes, shared texts and the mods
/// with an entry, in the order they start.
pub fn deploy_table(comp: &Composition, texts: Value) -> Result<Value> {
    table_with(comp, texts, read_texts)
}

/// Each catalog in a mod's `locales` folder as the digest of its file, in place of its texts. The
/// folder is listed once: most mods have none, and asking for each language's file costs a call each.
fn texts_digests(dir: &Path) -> Result<Value> {
    let mut out = Map::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return Ok(Value::Object(out)) };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(lang) = name.strip_suffix(".json").filter(|l| LANGUAGES.contains(l)) else { continue };
        let d = crate::compose::cached_digest(&e.path())?;
        out.insert(lang.to_string(), Value::String(d.iter().map(|b| format!("{b:02x}")).collect()));
    }
    Ok(Value::Object(out))
}

/// `deploy_table` with each mod's texts from `mod_texts`, given the mod's `locales` folder.
fn table_with(comp: &Composition, texts: Value, mod_texts: fn(&Path) -> Result<Value>) -> Result<Value> {
    let mut session = Vec::new();
    let mut every = Map::new();
    let mut expansions = Map::new();
    let mut expansion_tags = Map::new();
    let mut mods = Vec::new();
    let (mut has_session, mut has_static_session) = (false, false);
    let mut static_session = Vec::new();
    let libraries = session_libraries(&comp.mods);
    let served = expansion_libraries(&comp.mods);
    for c in start_order(&comp.mods) {
        let m = &c.manifest;
        let specs = m.option_specs()?;
        // A session mod's library is a session mod, and costs the deploy anyway. One only expansions
        // need follows their content. A mod with secondary scopes runs with the scope of those that are on.
        let scope = match m.scope_with(&c.values) {
            Scope::Library if libraries.contains(m.id.as_str()) => Scope::Session,
            Scope::Library if served.contains_key(m.id.as_str()) => Scope::Expansion,
            Scope::Library => Scope::Client,
            scope => scope,
        };
        let hybrid = scope == Scope::Session && m.hybrid_with(&c.values);
        let line = || SessionMod {
            id: m.id.clone(),
            version: m.version.clone(),
            options: specs.iter().map(|s| (s.id.clone(), c.values[&s.id].clone(), m.option_personal(s, &c.values))).collect(),
        };
        // An expansion with a session secondary scope on changes the shared game outside the content
        // too: the deploy counts it as a session mod, with the files of those secondary scopes.
        let outside = m.session_outside_content(&c.values);
        if scope == Scope::Session || outside {
            has_session = true;
            // The files of client secondary scopes are this player's own, as a client mod's are.
            let shared_files = if outside {
                m.resources.iter().filter(|r| m.tag_scope(r.part.as_deref(), &c.values) == Some(Scope::Session) && r.part.is_some()).count()
            } else {
                c.resources - c.client_resources
            };
            has_static_session |= shared_files > 0;
            let sm = line();
            if shared_files > 0 {
                static_session.push(sm.clone());
            }
            if m.every_player {
                every.insert(m.id.clone(), Value::String(every_hash(&sm)));
            }
            session.push(sm);
        }
        // What every player in the lobby has to run for the content to start: the same version and
        // shared option values, as for a mod marked every_player.
        let gate: Vec<&str> = if m.scope == Scope::Expansion {
            let hash = every_hash(&line());
            expansion_tags.insert(m.id.clone(), json!(expansion_pool_tag(&m.id, &hash)));
            expansions.insert(m.id.clone(), Value::String(hash));
            vec![m.id.as_str()]
        } else {
            served.get(m.id.as_str()).map(|ids| ids.iter().copied().collect()).unwrap_or_default()
        };
        if m.entry.is_none() {
            continue;
        }
        let options: Vec<Value> = specs
            .iter()
            .map(|s| {
                let mut o = json!({ "id": s.id, "type": s.kind.name(), "default": s.default, "client": s.client });
                match &s.kind {
                    crate::options::OptionKind::Number { min, max, step } => {
                        for (k, v) in [("min", min), ("max", max), ("step", step)] {
                            if let Some(v) = v {
                                o[k] = json!(v);
                            }
                        }
                    }
                    crate::options::OptionKind::Choice(ids) => o["choices"] = json!(ids),
                    _ => {}
                }
                o
            })
            .collect();
        let keybinds: Vec<Value> =
            crate::options::parse_keybinds(&m.keybinds)?.into_iter().map(|k| json!({ "id": k.id, "default": k.default })).collect();
        let mut entry = json!({
            "id": m.id,
            // By language, for obsidian.info: Obsidian API names each mod's folder of the in-game window with it.
            "name": m.name,
            "version": m.version.to_string(),
            // Hybrid: the session secondary scopes stop in a lobby that isn't cleared, the client ones keep running.
            "scope": if hybrid { "hybrid" } else { scope_text(&scope) },
            "entry": entry_name(&m.id),
            "loader_api": m.loader_api,
            "requires": m.requires,
            "options": options,
            "keybinds": keybinds,
            "texts": mod_texts(&c.dir.join("locales"))?,
        });
        // The expansions whose content switches this mod's gated code on: an expansion's own id, or
        // the expansions that require a library.
        if !gate.is_empty() {
            entry["expansions"] = json!(gate);
        }
        // Each secondary scope with the scope its code runs with, and whether it is on. The name, by
        // language, is for the notice when the session ones stop.
        if !m.parts.is_empty() {
            entry["parts"] = m.parts.iter().map(|p| json!({ "id": p.id, "scope": p.scope.name(), "on": m.part_on(p, &c.values), "name": p.name })).collect();
        }
        mods.push(entry);
    }
    let modset = modset_hash(&session);
    let static_hash = (!static_session.is_empty()).then(|| modset_hash(&static_session));
    let mut hashes = json!({ "open": open_pool_tag(), "sandbox": sandbox_pool_tag(static_hash.as_deref()) });
    if !every.is_empty() {
        hashes["every"] = json!(every_pool_tag(every.iter().map(|(id, h)| (id.as_str(), h.as_str().unwrap_or_default()))));
    }
    if !expansion_tags.is_empty() {
        hashes["expansions"] = Value::Object(expansion_tags);
    }
    let (font_files, font_paths) = deployed_fonts(comp);
    Ok(json!({
        "version": 1,
        "revision": REVISION,
        "modset": modset,
        "hashes": hashes,
        "every": every,
        "expansions": expansions,
        "has_session": has_session,
        "has_static_session": has_static_session,
        "font_files": font_files,
        "font_paths": font_paths,
        "texts": texts,
        "mods": mods,
    }))
}

/// A Lua literal for a JSON value. Strings escape every byte outside printable ASCII as `\ddd`.
pub fn to_lua(v: &Value) -> String {
    match v {
        Value::Null => "nil".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            let mut out = String::from("\"");
            for b in s.bytes() {
                match b {
                    b'\\' => out.push_str("\\\\"),
                    b'"' => out.push_str("\\\""),
                    0x20..=0x7e => out.push(b as char),
                    _ => out.push_str(&format!("\\{b:03}")),
                }
            }
            out.push('"');
            out
        }
        Value::Array(items) => format!("{{{}}}", items.iter().map(to_lua).collect::<Vec<_>>().join(", ")),
        Value::Object(map) => {
            format!("{{{}}}", map.iter().map(|(k, v)| format!("[{}] = {}", to_lua(&Value::String(k.clone())), to_lua(v))).collect::<Vec<_>>().join(", "))
        }
    }
}

/// Every loader module (`boot.lua` as the boot entry), every mod entry and `obsidian/deploy`.
pub fn build(loader_dir: &Path, comp: &Composition) -> Result<Vec<Planned>> {
    let mut files: Vec<PathBuf> =
        std::fs::read_dir(loader_dir)?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "lua")).collect();
    files.sort();
    if !files.iter().any(|p| p.file_stem().is_some_and(|s| s == "boot")) {
        return Err(ObsidianError::LoaderMissing(loader_dir.display().to_string()));
    }
    let mut scripts = Vec::new();
    for p in files {
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let module = format!("obsidian/loader/{stem}");
        let name = if stem == "boot" { ENTRY.to_string() } else { module.clone() };
        scripts.push((name, Script::Source(std::fs::read(&p)?, module), LOADER_OWNER.to_string()));
    }
    for c in &comp.mods {
        if let Some(entry) = &c.manifest.entry {
            let name = entry_name(&c.manifest.id);
            let script = entry_script(std::fs::read(c.dir.join(entry))?, &name)?;
            // A mod's scripts are that mod's, so the Deployed column counts them.
            scripts.push((name, script, c.manifest.id.clone()));
            for p in &c.manifest.modules {
                let name = module_name(&c.manifest.id, p);
                let script = entry_script(std::fs::read(c.dir.join(p))?, &name)?;
                scripts.push((name, script, c.manifest.id.clone()));
            }
        }
    }
    let source = format!("return {}\n", to_lua(&full_table(loader_dir, comp)?));
    scripts.push(("obsidian/deploy".to_string(), Script::Source(source.into_bytes(), "obsidian/deploy".to_string()), LOADER_OWNER.to_string()));

    let sources: Vec<(&[u8], &str)> = scripts
        .iter()
        .filter_map(|(_, s, _)| match s {
            Script::Source(source, chunk) => Some((source.as_slice(), chunk.as_str())),
            Script::Bytecode(_) => None,
        })
        .collect();
    let mut compiled = compile_all(&sources)?.into_iter();
    let mut planned: Vec<Planned> = scripts
        .iter()
        .map(|(name, s, owner)| match s {
            Script::Bytecode(code) => lua_resource(name, code, owner),
            Script::Source(..) => lua_resource(name, &compiled.next().unwrap(), owner),
        })
        .collect();
    for (kind, name, path) in own_fonts(loader_dir) {
        planned.push(Planned::new(hash_name(kind), hash_name(&name), 0, std::fs::read(path)?, LOADER_OWNER.into()));
    }
    Ok(planned)
}

/// What `build` compiles, uncompiled: the loader sources, each mod entry and the deploy table.
/// Equal inputs build equal resources, so this stands in for `build` where nothing is written.
pub fn build_inputs(loader_dir: &Path, comp: &Composition) -> Result<Vec<u8>> {
    // Each file goes in as its digest, kept while the file is unchanged: this runs for every snapshot,
    // after every click, and reading every script and font each time took tens of milliseconds.
    let digest = |p: &Path| crate::compose::cached_digest(p);
    let mut files: Vec<PathBuf> =
        std::fs::read_dir(loader_dir)?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "lua")).collect();
    files.sort();
    let mut out = Vec::new();
    for p in files {
        out.extend(p.file_name().unwrap().to_string_lossy().as_bytes());
        out.extend(digest(&p)?);
    }
    for c in &comp.mods {
        if let Some(entry) = &c.manifest.entry {
            out.extend(c.manifest.id.as_bytes());
            out.extend(digest(&c.dir.join(entry))?);
            for p in &c.manifest.modules {
                out.extend(digest(&c.dir.join(p))?);
            }
        }
    }
    for (_, name, path) in own_fonts(loader_dir) {
        out.extend(name.as_bytes());
        out.extend(digest(&path)?);
    }
    // The table the loader gets, with its texts as the digests of the files they come from: parsing
    // the catalogs every time cost more than everything else here.
    let locales = loader_dir.join("..").join("..").join("locales");
    for lang in LANGUAGES {
        let path = locales.join(format!("{lang}.json"));
        if path.is_file() {
            out.extend(lang.as_bytes());
            out.extend(digest(&path)?);
        }
    }
    out.extend(table_with(comp, Value::Null, texts_digests)?.to_string().as_bytes());
    Ok(out)
}

/// The settings file for this profile: consent, the guardrails and the value of every option of each
/// enabled mod with an entry. Reads manifests only, so nothing is compiled.
pub fn settings_for(lib: &Library, profile: &Profile) -> Result<Settings> {
    let sandbox = crate::saves::active_sandbox(lib.data_dir());
    let war = crate::commander::war_line(lib.data_dir(), sandbox.as_deref());
    let mut s = settings_from(profile, sandbox, |id| load_manifest(&lib.mod_dir(id)).ok().map(Cow::Owned))?;
    s.war = war;
    Ok(s)
}

/// `settings_for` with the manifests already read, as the app's snapshot has them.
pub fn settings_with(profile: &Profile, manifests: &BTreeMap<String, Manifest>) -> Result<Settings> {
    settings_from(profile, None, |id| manifests.get(id).map(Cow::Borrowed))
}

fn settings_from<'a>(profile: &Profile, sandbox: Option<String>, manifest: impl Fn(&str) -> Option<Cow<'a, Manifest>>) -> Result<Settings> {
    let mut s = Settings {
        consent: profile.allows_others(),
        block_achievements: profile.block_achievements,
        freeze_progress: profile.freeze_progress,
        sandbox,
        hide_client_mods: profile.hide_client_mods,
        ..Default::default()
    };
    for entry in profile.mods.iter().filter(|m| m.enabled) {
        // A mod that can't be read stays out, as it does in a deploy.
        let Some(m) = manifest(&entry.id) else { continue };
        if m.entry.is_none() {
            continue;
        }
        let specs = m.option_specs()?;
        let values: BTreeMap<String, Value> = effective_values(&specs, profile.options.get(&m.id));
        for spec in &specs {
            s.options.push((m.id.clone(), spec.id.clone(), spec.kind.name().to_string(), value_text(&values[&spec.id])));
        }
        // Only keybinds the manifest still declares, so a key set for an older version is dropped.
        let set = profile.keys.get(&m.id);
        for k in crate::options::parse_keybinds(&m.keybinds)? {
            if let Some(key) = set.and_then(|s| s.get(&k.id)) {
                s.keys.push((m.id.clone(), k.id, key.clone()));
            }
        }
    }
    Ok(s)
}

/// The loader's lines that tell a normal exit from a crash, the first written once it can tell.
const WATCHING: &str = "watching for the game to close";
const CLOSED: &str = "game closed";

/// The last line of a log whose game stopped without closing: a crash, or the process being killed.
/// A log from before the loader could tell says nothing.
pub fn unclean_end(log: &str) -> Option<String> {
    let text = |line: &'_ str| line.split_once(' ').map_or(line, |(_, rest)| rest).trim().to_string();
    let lines: Vec<&str> = log.lines().filter(|l| !l.trim().is_empty()).collect();
    if !lines.iter().any(|l| text(l) == WATCHING) {
        return None;
    }
    lines.last().map(|l| text(l)).filter(|last| last != CLOSED)
}

/// The mod named by a `loading` line that no `started`, `failed` or `refused` line followed: the
/// mod that was starting when the game last closed or crashed.
pub fn crash_culprit(log: &str) -> Option<String> {
    let mut pending: Option<String> = None;
    for line in log.lines() {
        let text = line.split_once(' ').map_or(line, |(_, rest)| rest);
        if let Some(id) = text.strip_prefix("loading ") {
            pending = Some(id.trim().to_string());
        } else if let Some(rest) = ["started ", "failed ", "refused "].iter().find_map(|p| text.strip_prefix(p)) {
            let id = rest.split(':').next().unwrap_or("").trim();
            if pending.as_deref() == Some(id) {
                pending = None;
            }
        }
    }
    pending
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::compose;
    use crate::library::ProfileEntry;
    use crate::testutil::{put, spaced_tempdir};

    fn add(t: &Path, lib: &Library, id: &str, scope: &str) {
        let d = t.join(format!("stage-{id}"));
        let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},"scope":"{scope}","entry":"main.lua"}}"#);
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "main.lua", b"return true\n");
        lib.install(&d).unwrap();
    }

    #[test]
    fn deploy_table_lists_mods_in_load_order() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add(t.path(), &lib, "a", "client");
        add(t.path(), &lib, "b", "session");
        let entry = |id: &str| ProfileEntry { id: id.into(), enabled: true, folder: None };
        let profile = Profile { mods: vec![entry("b"), entry("a")], ..Default::default() };
        let comp = compose(&lib, &profile).unwrap();
        let table = deploy_table(&comp, json!({})).unwrap();
        let ids: Vec<&str> = table["mods"].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["b", "a"]);
        assert_eq!(table["mods"][0]["entry"], "obsidian/mods/b/main");
        assert_eq!(table["mods"][0]["name"], json!({ "en": "b" }), "the name, by language, for obsidian.info");
        assert_eq!(table["has_session"], true);
        assert_eq!(table["has_static_session"], false, "an entry can stop when the session isn't cleared");
        assert_eq!(table["modset"].as_str().unwrap().len(), 64);
        assert_eq!(table["hashes"]["sandbox"], crate::modset::sandbox_pool_tag(None), "no static session content");
        let mut with_static = comp.clone();
        with_static.mods.iter_mut().find(|m| m.manifest.id == "b").unwrap().resources = 1;
        let folded = deploy_table(&with_static, json!({})).unwrap();
        assert_ne!(folded["hashes"]["sandbox"], table["hashes"]["sandbox"], "static session content is folded in");
        let planned = build(&find_loader_dir().unwrap(), &comp).unwrap();
        for name in [ENTRY, "obsidian/loader/core", "obsidian/mods/a/main", "obsidian/mods/b/main", "obsidian/deploy"] {
            assert!(planned.iter().any(|p| p.name_hash == hash_name(name)), "{name}");
        }
        // Each script belongs to the mod it comes from, so the deploy counts its bytes for that mod.
        let owner = |name: &str| planned.iter().find(|p| p.name_hash == hash_name(name)).unwrap().owner.clone();
        assert_eq!([owner("obsidian/mods/a/main"), owner("obsidian/mods/b/main")], ["a", "b"]);
        assert_eq!([owner(ENTRY), owner("obsidian/deploy")], [LOADER_OWNER, LOADER_OWNER]);
    }

    #[test]
    fn a_selected_sandbox_needs_the_loader() {
        let comp = Composition::default();
        assert!(!needed(&comp, &Profile::default()));
        assert!(needed(&Composition { sandbox: true, ..comp }, &Profile::default()));
    }

    fn add_requiring(t: &Path, lib: &Library, id: &str, scope: &str, requires: &str) {
        let d = t.join(format!("stage-{id}"));
        let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},"scope":"{scope}","entry":"main.lua","requires":["{requires}"]}}"#);
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "main.lua", b"return true\n");
        lib.install(&d).unwrap();
    }

    fn table_of(lib: &Library, ids: &[&str]) -> Value {
        let entries = ids.iter().map(|id| ProfileEntry { id: (*id).into(), enabled: true, folder: None }).collect();
        let profile = Profile { mods: entries, ..Default::default() };
        deploy_table(&compose(lib, &profile).unwrap(), json!({})).unwrap()
    }

    #[test]
    fn a_mod_every_player_needs_gets_its_hash_and_a_pool_of_its_own() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add(t.path(), &lib, "plain", "session");
        let d = t.path().join("stage-big");
        put(&d, "obsidian.json", br#"{"obsidian":1,"id":"big","version":"1.0.0","name":{"en":"big"},"scope":"library","every_player":true,"entry":"main.lua"}"#);
        put(&d, "main.lua", b"return true
");
        lib.install(&d).unwrap();
        add_requiring(t.path(), &lib, "user", "session", "big");

        let plain = table_of(&lib, &["plain"]);
        assert_eq!(plain["every"], json!({}));
        assert!(plain["hashes"].get("every").is_none(), "the open pool");

        let alone = table_of(&lib, &["big"]);
        assert_eq!(alone["every"], json!({}), "a library no session mod needs runs as a client mod");

        let table = table_of(&lib, &["plain", "big", "user"]);
        let hash = table["every"]["big"].as_str().unwrap();
        assert_eq!(hash.len(), 16);
        assert_eq!(table["every"].as_object().unwrap().len(), 1);
        assert_eq!(table["hashes"]["every"], every_pool_tag([("big", hash)]));
        assert_eq!(table["hashes"]["open"], open_pool_tag(), "kept for the bridge to the open pool");
    }

    fn scope_in<'a>(table: &'a Value, id: &str) -> &'a str {
        table["mods"].as_array().unwrap().iter().find(|m| m["id"] == id).unwrap()["scope"].as_str().unwrap()
    }

    #[test]
    fn a_mod_with_no_scope_runs_as_a_client_mod() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage-legacy");
        put(&d, "obsidian.json", br#"{"obsidian":1,"id":"legacy","version":"1.0.0","name":{"en":"legacy"},"entry":"main.lua"}"#);
        put(&d, "main.lua", b"return true\n");
        lib.install(&d).unwrap();
        let table = table_of(&lib, &["legacy"]);
        assert_eq!(scope_in(&table, "legacy"), "client");
        assert_eq!(table["has_session"], false, "it keeps the player in the normal pool");
    }

    #[test]
    fn a_library_alone_or_with_client_mods_runs_as_a_client_mod() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add(t.path(), &lib, "lib", "library");
        add_requiring(t.path(), &lib, "skin", "client", "lib");
        let vanilla = table_of(&lib, &[]);
        for ids in [&["lib"][..], &["lib", "skin"][..]] {
            let table = table_of(&lib, ids);
            assert_eq!(scope_in(&table, "lib"), "client");
            assert_eq!(table["has_session"], false);
            assert_eq!(table["modset"], vanilla["modset"], "it stays out of the sets players compare");
        }
    }

    #[test]
    fn a_library_runs_as_a_session_mod_when_a_session_mod_needs_it() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add(t.path(), &lib, "base", "library");
        add_requiring(t.path(), &lib, "mid", "library", "base");
        add_requiring(t.path(), &lib, "cheat", "session", "mid");
        add_requiring(t.path(), &lib, "skin", "client", "base");
        let table = table_of(&lib, &["base", "mid", "cheat", "skin"]);
        assert_eq!(scope_in(&table, "base"), "session", "through the other library");
        assert_eq!(scope_in(&table, "mid"), "session");
        assert_eq!(scope_in(&table, "skin"), "client");
        assert_eq!(table["has_session"], true);
    }

    #[test]
    fn an_expansion_deploys_as_a_client_mod_with_its_hash_and_pool_tag() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage-arcade");
        put(&d, "obsidian.json", br#"{"obsidian":1,"id":"arcade","version":"1.0.0","name":{"en":"Arcade"},"scope":"expansion","entry":"main.lua",
            "options":[{"id":"waves","type":"number","default":5},{"id":"cam","type":"number","default":1,"scope":"client"}]}"#);
        put(&d, "main.lua", b"return true\n");
        lib.install(&d).unwrap();
        add(t.path(), &lib, "skin", "client");
        let vanilla = table_of(&lib, &[]);
        let table = table_of(&lib, &["arcade", "skin"]);
        assert_eq!(scope_in(&table, "arcade"), "expansion");
        assert_eq!(table["has_session"], false, "it moves nobody out of the normal pool");
        assert_eq!(table["modset"], vanilla["modset"], "and stays out of the sets session mods compare");
        assert_eq!(table["every"], json!({}));
        let hash = table["expansions"]["arcade"].as_str().unwrap();
        assert_eq!(hash.len(), 16);
        assert_eq!(table["hashes"]["expansions"]["arcade"], expansion_pool_tag("arcade", hash));
        assert_eq!(table["mods"].as_array().unwrap().iter().find(|m| m["id"] == "arcade").unwrap()["expansions"], json!(["arcade"]));
        assert!(table["mods"].as_array().unwrap().iter().find(|m| m["id"] == "skin").unwrap().get("expansions").is_none());
        assert!(vanilla["hashes"].get("expansions").is_none());
        assert_eq!(vanilla["expansions"], json!({}));
    }

    #[test]
    fn an_expansion_hash_follows_the_version_and_shared_options_only() {
        let hash = |version: &str, waves: u32, cam: u32| {
            let t = spaced_tempdir();
            let lib = Library::open(&t.path().join("data")).unwrap();
            let d = t.path().join("stage-arcade");
            let json = format!(
                r#"{{"obsidian":1,"id":"arcade","version":"{version}","name":{{"en":"Arcade"}},"scope":"expansion","entry":"main.lua",
                "options":[{{"id":"waves","type":"number","default":{waves}}},{{"id":"cam","type":"number","default":{cam},"scope":"client"}}]}}"#
            );
            put(&d, "obsidian.json", json.as_bytes());
            put(&d, "main.lua", b"return true\n");
            lib.install(&d).unwrap();
            table_of(&lib, &["arcade"])["expansions"]["arcade"].clone()
        };
        assert_eq!(hash("1.0.0", 5, 1), hash("1.0.0", 5, 9), "a personal option doesn't count");
        assert_ne!(hash("1.0.0", 5, 1), hash("1.0.0", 6, 1));
        assert_ne!(hash("1.0.0", 5, 1), hash("1.0.1", 5, 1));
    }

    #[test]
    fn a_library_only_expansions_need_follows_their_content() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add(t.path(), &lib, "base", "library");
        add_requiring(t.path(), &lib, "mid", "library", "base");
        add_requiring(t.path(), &lib, "arcade", "expansion", "mid");
        add_requiring(t.path(), &lib, "cheat", "session", "base");
        let mods = |table: &Value, id: &str| table["mods"].as_array().unwrap().iter().find(|m| m["id"] == id).unwrap().clone();

        let table = table_of(&lib, &["base", "mid", "arcade"]);
        assert_eq!(scope_in(&table, "base"), "expansion", "through the other library");
        assert_eq!(scope_in(&table, "mid"), "expansion");
        assert_eq!(mods(&table, "base")["expansions"], json!(["arcade"]));
        assert_eq!(table["has_session"], false);

        let with_session = table_of(&lib, &["base", "mid", "arcade", "cheat"]);
        assert_eq!(scope_in(&with_session, "base"), "session", "a session mod needs it too");
        assert_eq!(mods(&with_session, "base")["expansions"], json!(["arcade"]), "and the expansion still gets full use");
        assert_eq!(scope_in(&with_session, "mid"), "expansion");
        assert_eq!(with_session["has_session"], true);
    }

    #[test]
    fn a_library_version_counts_only_when_it_runs_as_a_session_mod() {
        let hash = |version: &str, with_session_mod: bool| {
            let t = spaced_tempdir();
            let lib = Library::open(&t.path().join("data")).unwrap();
            let d = t.path().join("stage-lib");
            put(&d, "obsidian.json", format!(r#"{{"obsidian":1,"id":"lib","version":"{version}","name":{{"en":"lib"}},"scope":"library","entry":"main.lua"}}"#).as_bytes());
            put(&d, "main.lua", b"return true\n");
            lib.install(&d).unwrap();
            add_requiring(t.path(), &lib, "cheat", "session", "lib");
            let ids: &[&str] = if with_session_mod { &["lib", "cheat"] } else { &["lib"] };
            table_of(&lib, ids)["modset"].clone()
        };
        assert_ne!(hash("1.0.0", true), hash("1.0.1", true));
        assert_eq!(hash("1.0.0", false), hash("1.0.1", false));
    }

    #[test]
    fn a_mod_with_no_parts_hashes_and_deploys_as_before_parts() {
        use sha2::{Digest, Sha256};
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage-plain");
        put(&d, "obsidian.json", br#"{"obsidian":1,"id":"plain","version":"1.2.0","name":{"en":"Plain"},"scope":"session","entry":"main.lua",
            "options":[{"id":"speed","type":"number","default":5},{"id":"cam","type":"toggle","default":true,"scope":"client"}],
            "resources":[{"type":"texture","name":"a/b","bundle":"00000000000000aa","data":"f"}]}"#);
        put(&d, "main.lua", b"return true\n");
        put(&d, "f", b"x");
        lib.install(&d).unwrap();
        let table = table_of(&lib, &["plain"]);
        // The line and the hash as they were written before parts: id@version and the shared options.
        let want: String = Sha256::digest(b"plain@1.2.0;speed=5").iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(table["modset"], json!(want));
        assert_eq!((table["has_session"].clone(), table["has_static_session"].clone()), (json!(true), json!(true)));
        let entry = &table["mods"][0];
        assert_eq!(entry["scope"], "session");
        assert!(entry.get("parts").is_none());
        let keys: Vec<&str> = entry.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["entry", "id", "keybinds", "loader_api", "name", "options", "requires", "scope", "texts", "version"]);
        assert_eq!(entry["options"][1], json!({ "id": "cam", "type": "toggle", "default": true, "client": true }));
    }

    /// A mod like Integrated Third Person, with the option `far` switching the session part, and a
    /// library it requires.
    fn hybrid_library(t: &Path) -> Library {
        let lib = Library::open(&t.join("data")).unwrap();
        add(t, &lib, "lib", "library");
        let d = t.join("stage-cam");
        put(&d, "obsidian.json", br#"{"obsidian":1,"id":"cam","version":"1.0.0","name":{"en":"Cam"},"scope":"session","entry":"main.lua","requires":["lib"],
            "parts":[{"id":"view","scope":"client","name":{"en":"View"}},{"id":"range","scope":"session","option":"far","name":{"en":"Range"}}],
            "options":[{"id":"fov","type":"number","default":70,"part":"view"},{"id":"far","type":"toggle","default":true,"part":"range"},
                       {"id":"power","type":"number","default":1,"part":"range"}],
            "resources":[{"type":"texture","name":"cam/sky","bundle":"00000000000000aa","data":"f","part":"view"},
                         {"type":"texture","name":"cam/range","bundle":"00000000000000aa","data":"f","part":"range"}]}"#);
        put(&d, "main.lua", b"return true\n");
        put(&d, "f", b"x");
        lib.install(&d).unwrap();
        lib
    }

    fn hybrid_table(lib: &Library, values: &[(&str, Value)]) -> (Value, Composition) {
        let entries = ["lib", "cam"].iter().map(|id| ProfileEntry { id: (*id).into(), enabled: true, folder: None }).collect();
        let mut profile = Profile { mods: entries, ..Default::default() };
        for (k, v) in values {
            profile.options.entry("cam".into()).or_default().insert((*k).into(), v.clone());
        }
        let comp = compose(lib, &profile).unwrap();
        (deploy_table(&comp, json!({})).unwrap(), comp)
    }

    #[test]
    fn a_hybrid_mod_deploys_its_parts_and_the_library_it_needs_as_session() {
        let t = spaced_tempdir();
        let lib = hybrid_library(t.path());
        let (table, comp) = hybrid_table(&lib, &[]);
        assert_eq!(scope_in(&table, "cam"), "hybrid");
        assert_eq!(scope_in(&table, "lib"), "session", "the session part may call guarded functions");
        let cam = table["mods"].as_array().unwrap().iter().find(|m| m["id"] == "cam").unwrap();
        assert_eq!(cam["parts"], json!([
            { "id": "view", "scope": "client", "on": true, "name": { "en": "View" } },
            { "id": "range", "scope": "session", "on": true, "name": { "en": "Range" } },
        ]));
        assert_eq!((table["has_session"].clone(), table["has_static_session"].clone()), (json!(true), json!(true)));
        assert_eq!(comp.bundles["00000000000000aa"].len(), 2);

        let (fov, _) = hybrid_table(&lib, &[("fov", json!(90))]);
        assert_eq!(fov["modset"], table["modset"], "a client part's option stays out of the hash");
        let (power, _) = hybrid_table(&lib, &[("power", json!(2))]);
        assert_ne!(power["modset"], table["modset"]);
    }

    #[test]
    fn a_session_part_that_is_off_counts_for_nothing() {
        let t = spaced_tempdir();
        let lib = hybrid_library(t.path());
        let (table, comp) = hybrid_table(&lib, &[("far", json!(false))]);
        assert_eq!(scope_in(&table, "cam"), "client");
        assert_eq!(scope_in(&table, "lib"), "client");
        assert_eq!(table["has_session"], false, "Galactic War results go out");
        assert_eq!(table["has_static_session"], false);
        assert_eq!(table["modset"], table_of(&lib, &[])["modset"], "as a client mod");
        let cam = table["mods"].as_array().unwrap().iter().find(|m| m["id"] == "cam").unwrap();
        assert_eq!(cam["parts"][1]["on"], false);
        assert_eq!(comp.bundles["00000000000000aa"].len(), 1, "the part's file stays out");
        let (power, _) = hybrid_table(&lib, &[("far", json!(false)), ("power", json!(2))]);
        assert_eq!(power["modset"], table["modset"]);
    }

    #[test]
    fn the_files_of_a_client_part_keep_the_public_door_open() {
        let t = spaced_tempdir();
        let lib = hybrid_library(t.path());
        let json = std::fs::read_to_string(lib.mod_dir("cam").join("obsidian.json")).unwrap();
        let d = t.path().join("stage-cam2");
        let view_only = json.replace(r#""name":"cam/range","bundle":"00000000000000aa","data":"f","part":"range""#, r#""name":"cam/sky2","bundle":"00000000000000aa","data":"f","part":"view""#);
        assert_ne!(view_only, json);
        put(&d, "obsidian.json", view_only.as_bytes());
        put(&d, "main.lua", b"return true\n");
        put(&d, "f", b"x");
        lib.install(&d).unwrap();
        let (table, _) = hybrid_table(&lib, &[]);
        assert_eq!((table["has_session"].clone(), table["has_static_session"].clone()), (json!(true), json!(false)));
    }

    #[test]
    fn an_expansion_with_a_session_secondary_scope_on_deploys_as_session_content_and_stays_an_expansion() {
        let t = spaced_tempdir();
        let lib = hybrid_library(t.path());
        let json = std::fs::read_to_string(lib.mod_dir("cam").join("obsidian.json")).unwrap();
        let expansion = json
            .replacen(r#""scope":"session","entry""#, r#""scope":"expansion","entry""#, 1)
            .replace(r#""parts":"#, r#""secondary_scopes":"#)
            .replace(r#""part":"#, r#""secondary_scope":"#);
        assert!(expansion.contains(r#""scope":"expansion""#) && expansion.contains("secondary_scopes"), "{expansion}");
        let d = t.path().join("stage-cam3");
        put(&d, "obsidian.json", expansion.as_bytes());
        put(&d, "main.lua", b"return true\n");
        put(&d, "f", b"x");
        lib.install(&d).unwrap();

        let (on, _) = hybrid_table(&lib, &[]);
        assert_eq!(scope_in(&on, "cam"), "expansion", "the loader keeps the content gate");
        assert_eq!(scope_in(&on, "lib"), "session", "the session secondary scope may call guarded functions outside the content");
        assert_eq!((on["has_session"].clone(), on["has_static_session"].clone()), (json!(true), json!(true)), "its file changes the shared game");
        let cam = on["mods"].as_array().unwrap().iter().find(|m| m["id"] == "cam").unwrap();
        assert_eq!(cam["parts"][1], json!({ "id": "range", "scope": "session", "on": true, "name": { "en": "Range" } }));

        let (off, _) = hybrid_table(&lib, &[("far", json!(false))]);
        assert_eq!(scope_in(&off, "cam"), "expansion");
        assert_eq!(off["has_session"], false, "with the session secondary scope off, an expansion is an expansion");
    }

    #[test]
    fn the_loader_brings_its_fonts_and_tells_the_guis() {
        let dir = find_loader_dir().unwrap();
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let comp = compose(&lib, &Profile::default()).unwrap();
        let table = full_table(&dir, &comp).unwrap();
        assert_eq!(table["font_files"], json!(["obsidian/fonts/fonts"]));
        assert_eq!(table["font_paths"], json!(["obsidian/fonts/body", "obsidian/fonts/heading"]));
        let planned = build(&dir, &comp).unwrap();
        for (kind, name) in [("font", "obsidian/fonts/body"), ("texture", "obsidian/fonts/heading"), ("material", "obsidian/fonts/fonts")] {
            assert!(planned.iter().any(|p| p.type_hash == hash_name(kind) && p.name_hash == hash_name(name)), "{name}");
        }
    }

    #[test]
    fn a_changed_font_counts_as_a_change_to_deploy() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let comp = compose(&lib, &Profile::default()).unwrap();
        let root = t.path().join("game");
        put(&root, "loader/src/boot.lua", b"return true\n");
        put(&root, "locales/en.json", b"{}");
        let dir = root.join("loader").join("src");
        let before = build_inputs(&dir, &comp).unwrap();
        put(&root, "loader/fonts/body.font", b"one");
        let with_font = build_inputs(&dir, &comp).unwrap();
        // Another size: the digest cache knows a file by its time and size, and two writes in a row
        // can share a time on a fast machine.
        put(&root, "loader/fonts/body.font", b"three");
        assert_ne!(before, with_font);
        assert_ne!(with_font, build_inputs(&dir, &comp).unwrap());
    }

    #[test]
    fn a_changed_loader_script_counts_as_a_change_to_deploy() {
        // The game-side folder moved with the rename of the product: a loader deployed before it
        // reads the old folder, so the first deploy after the upgrade must be asked for.
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let comp = compose(&lib, &Profile::default()).unwrap();
        let root = t.path().join("game");
        put(&root, "loader/src/core.lua", b"local dir = 'ObsidianModManager'\n");
        put(&root, "locales/en.json", b"{}");
        let dir = root.join("loader").join("src");
        let before = build_inputs(&dir, &comp).unwrap();
        put(&root, "loader/src/core.lua", b"local dir = 'ObsidianLauncher'\n");
        assert_ne!(before, build_inputs(&dir, &comp).unwrap());
    }

    #[test]
    fn only_fonts_in_the_boot_bundle_are_offered() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage");
        let json = format!(
            r#"{{"obsidian":1,"id":"a.fonts","version":"1.0.0","name":{{"en":"x"}},"scope":"client","entry":"main.lua","options":[{{"id":"extra","type":"toggle","default":false}}],"resources":[
            {{"type":"font","name":"a/fonts/good","bundle":"{BUNDLE}","data":"f"}},
            {{"type":"material","name":"a/fonts/fonts","bundle":"{BUNDLE}","data":"f"}},
            {{"type":"font","name":"a/fonts/elsewhere","bundle":"00000000000000aa","data":"f"}},
            {{"type":"font","name":"a/fonts/off","bundle":"{BUNDLE}","data":"f","when":{{"extra":true}}}}]}}"#
        );
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "main.lua", b"return true\n");
        put(&d, "f", b"x");
        lib.install(&d).unwrap();
        let profile = Profile { mods: vec![ProfileEntry { id: "a.fonts".into(), enabled: true, folder: None }], ..Default::default() };
        let table = deploy_table(&compose(&lib, &profile).unwrap(), json!({})).unwrap();
        assert_eq!(table["font_files"], json!(["a/fonts/fonts"]));
        assert_eq!(table["font_paths"], json!(["a/fonts/good"]), "what isn't loaded for sure stays out");
    }

    #[test]
    fn a_boot_material_written_by_hash_is_still_a_material() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let d = t.path().join("stage");
        let json = format!(
            r##"{{"obsidian":1,"id":"a.hash","version":"1.0.0","name":{{"en":"x"}},"entry":"main.lua","resources":[
            {{"type":"#{:016x}","name":"a/icons","bundle":"{BUNDLE}","data":"f"}}]}}"##,
            hash_name("material")
        );
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "main.lua", b"return true\n");
        put(&d, "f", b"x");
        lib.install(&d).unwrap();
        let profile = Profile { mods: vec![ProfileEntry { id: "a.hash".into(), enabled: true, folder: None }], ..Default::default() };
        let table = deploy_table(&compose(&lib, &profile).unwrap(), json!({})).unwrap();
        assert_eq!(table["font_files"], json!(["a/icons"]));
    }

    #[test]
    fn a_mods_catalogs_reach_the_game_in_every_language_the_game_has() {
        let t = spaced_tempdir();
        for (lang, text) in [("en", "hi"), ("ru", "privet"), ("zh-CN", "ni hao"), ("xx", "unknown")] {
            put(t.path(), &format!("locales/{lang}.json"), format!(r#"{{"hello":"{text}"}}"#).as_bytes());
        }
        let texts = read_texts(&t.path().join("locales")).unwrap();
        assert_eq!(texts["ru"]["hello"], "privet");
        assert_eq!(texts["zh-CN"]["hello"], "ni hao");
        assert!(texts.get("xx").is_none(), "a file that names no language of the game stays out");
    }

    #[test]
    fn modules_become_resources_and_required_mods_start_first() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        add(t.path(), &lib, "base", "session");
        let d = t.path().join("stage-user");
        let json = r#"{"obsidian":1,"id":"user","version":"1.0.0","name":{"en":"user"},"entry":"main.lua",
                       "modules":["lib/parts.lua"],"requires":["base"]}"#;
        put(&d, "obsidian.json", json.as_bytes());
        put(&d, "main.lua", b"return true\n");
        put(&d, "lib/parts.lua", b"return {}\n");
        lib.install(&d).unwrap();
        let entry = |id: &str| ProfileEntry { id: id.into(), enabled: true, folder: None };
        let profile = Profile { mods: vec![entry("user"), entry("base")], ..Default::default() };
        let comp = compose(&lib, &profile).unwrap();
        let table = deploy_table(&comp, json!({})).unwrap();
        let ids: Vec<&str> = table["mods"].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["base", "user"]);
        assert_eq!(table["mods"][1]["requires"], json!(["base"]));
        let planned = build(&find_loader_dir().unwrap(), &comp).unwrap();
        assert!(planned.iter().any(|p| p.name_hash == hash_name("obsidian/mods/user/lib/parts")));
    }

    #[test]
    fn the_starter_mods_compile_and_the_trainer_brings_the_api() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        let mut profile = Profile::default();
        crate::starter::restore(&lib, &mut profile).unwrap();
        assert!(profile.mods.iter().any(|m| m.id == "obsidian.api" && m.enabled), "Obsidian Patches brings the API");
        assert_eq!(profile.switch(&lib, "obsidian.trainer", true).unwrap(), Vec::<String>::new());
        let comp = compose(&lib, &profile).unwrap();
        let table = deploy_table(&comp, json!({})).unwrap();
        let ids: Vec<&str> = table["mods"].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["obsidian.api", "obsidian.trainer", "obsidian.qol"]);
        assert_eq!(table["has_session"], true, "session mods, so the shared war stays out of it");
        assert_eq!(table["has_static_session"], false, "both can switch off live");
        assert_eq!(table["mods"][0]["keybinds"][0]["default"], "f6", "the in-game window's key is Obsidian API's");
        assert_eq!(table["mods"][1]["keybinds"], json!([]), "the Trainer's pages open with it");
        assert_eq!(table["mods"][1]["name"]["en"], "Obsidian Trainer", "the name of the Trainer's folder");
        assert!(table["mods"][1]["texts"]["pt-BR"]["god"].is_string());
        let planned = build(&find_loader_dir().unwrap(), &comp).unwrap();
        for name in ["obsidian/loader/menu", "obsidian/loader/storage", "obsidian/mods/obsidian.api/lib/game", "obsidian/mods/obsidian.api/lib/data", "obsidian/mods/obsidian.trainer/pages/player"] {
            assert!(planned.iter().any(|p| p.name_hash == hash_name(name)), "{name}");
        }
    }

    #[test]
    fn settings_for_writes_keys_of_enabled_mods() {
        let t = spaced_tempdir();
        let lib = Library::open(&t.path().join("data")).unwrap();
        for id in ["k1", "k2"] {
            let d = t.path().join(format!("stage-{id}"));
            let json = format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"{id}"}},"entry":"main.lua","keybinds":[{{"id":"toggle","default":"f8"}}]}}"#);
            put(&d, "obsidian.json", json.as_bytes());
            put(&d, "main.lua", b"return true\n");
            lib.install(&d).unwrap();
        }
        let mut profile = Profile {
            mods: vec![ProfileEntry { id: "k1".into(), enabled: true, folder: None }, ProfileEntry { id: "k2".into(), enabled: false, folder: None }],
            ..Default::default()
        };
        for id in ["k1", "k2"] {
            profile.keys.entry(id.into()).or_default().insert("toggle".into(), "f9".into());
        }
        profile.keys.get_mut("k1").unwrap().insert("gone".into(), "f10".into());
        let s = settings_for(&lib, &profile).unwrap();
        assert_eq!(s.keys, vec![("k1".to_string(), "toggle".to_string(), "f9".to_string())]);
    }

    #[test]
    fn shared_texts_leave_out_app_keys() {
        let texts = read_texts(&find_loader_dir().unwrap().join("..").join("..").join("locales")).unwrap();
        let en = texts["en"].as_object().unwrap();
        assert!(en.contains_key("mod_failed"));
        assert!(en.keys().all(|k| !k.starts_with("app.")), "{:?}", en.keys().collect::<Vec<_>>());
    }

    #[test]
    fn lua_strings_escape_every_byte_outside_ascii() {
        assert_eq!(to_lua(&json!("a\"b\\c\né1")), r#""a\"b\\c\010\195\1691""#);
        assert_eq!(to_lua(&json!({"k": [1, true]})), r#"{["k"] = {1, true}}"#);
    }

    #[test]
    fn a_log_that_never_says_the_game_closed_ended_badly() {
        assert_eq!(unclean_end("1 boot done\n2 watching for the game to close\n3 session: x\n"), Some("session: x".into()));
        assert_eq!(unclean_end("1 watching for the game to close\n2 game closed\n"), None);
        assert_eq!(unclean_end("1 watching for the game to close\n2 game closed\n\n"), None, "a blank end");
        assert_eq!(unclean_end("1 boot done\n2 session: x\n"), None, "an older loader can't tell");
        assert_eq!(unclean_end(""), None);
    }

    #[test]
    fn crash_culprit_is_the_last_unfinished_loading_line() {
        assert_eq!(crash_culprit("1 loading a\n2 started a\n3 loading b\n"), Some("b".into()));
        assert_eq!(crash_culprit("1 loading a\n2 failed a: boom\n3 boot done\n"), None);
        assert_eq!(crash_culprit(""), None);
    }
}
