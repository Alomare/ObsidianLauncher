//! The LuaJIT 2.0 compiler that ships beside Obsidian: the game runs only LuaJIT 2.0 bytecode.

use std::path::PathBuf;
use std::process::Command;

use crate::lua::check_bytecode;
use crate::{ObsidianError, Result};

#[cfg(windows)]
const EXE: &str = "luajit.exe";
#[cfg(not(windows))]
const EXE: &str = "luajit";

/// Turns `0.lua`, `1.lua`... in the working folder into `0.ljbc`, `1.ljbc`..., the bytes `luajit -b -s`
/// writes. `chunks.txt` names them, one per line. A failure prints the file's number, then the message.
const BATCH: &str = r#"
local count, chunks = tonumber(arg[1]), {}
for line in io.lines("chunks.txt") do chunks[#chunks + 1] = line end
for i = 0, count - 1 do
  local file = assert(io.open(i .. ".lua", "rb"))
  local source = file:read("*a")
  file:close()
  local code, message = loadstring(source, "=" .. chunks[i + 1])
  if not code then
    io.stderr:write(i, "\n", message)
    os.exit(1)
  end
  file = assert(io.open(i .. ".ljbc", "wb"))
  file:write(string.dump(code, true))
  file:close()
end
"#;

/// `OBSIDIAN_LUAJIT`, then `luajit/` in the app's resources, beside the running exe, then the workspace
/// build in `target/luajit/`; the first one that exists.
pub fn find_compiler() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(p) = std::env::var_os("OBSIDIAN_LUAJIT") {
        candidates.push(PathBuf::from(p));
    }
    if let Some(dir) = crate::paths::resource_dir() {
        candidates.push(dir.join("luajit").join(EXE));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        candidates.push(dir.join("luajit").join(EXE));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/luajit").join(EXE));
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .ok_or_else(|| ObsidianError::CompilerUnavailable(candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")))
}

/// The app has no console of its own, so Windows would open one for each run of the compiler.
fn no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Compiles Lua source to stripped LuaJIT 2.0 bytecode. Errors name `chunk` instead of the temp file.
pub fn compile(source: &[u8], chunk: &str) -> Result<Vec<u8>> {
    Ok(compile_all(&[(source, chunk)])?.remove(0))
}

/// `compile` for several `(source, chunk)` in one run of the compiler, because starting it is what
/// takes the time. The results are in the same order.
pub fn compile_all(sources: &[(&[u8], &str)]) -> Result<Vec<Vec<u8>>> {
    // Equal source and chunk name compile to equal bytecode, so a script compiled once isn't compiled
    // again: a deploy compiles every mod script, and most are the same as at the last deploy.
    type Compiled = std::sync::Mutex<std::collections::HashMap<[u8; 32], Vec<u8>>>;
    static COMPILED: std::sync::OnceLock<Compiled> = std::sync::OnceLock::new();
    let compiled = COMPILED.get_or_init(Default::default);
    let key = |(source, chunk): &(&[u8], &str)| -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(chunk.as_bytes());
        h.update([0]);
        h.update(source);
        h.finalize().into()
    };
    let keys: Vec<[u8; 32]> = sources.iter().map(key).collect();
    let mut out: Vec<Option<Vec<u8>>> = {
        let known = compiled.lock().unwrap_or_else(|p| p.into_inner());
        keys.iter().map(|k| known.get(k).cloned()).collect()
    };
    let missing: Vec<usize> = (0..sources.len()).filter(|i| out[*i].is_none()).collect();
    let fresh = run_compiler(&missing.iter().map(|i| sources[*i]).collect::<Vec<_>>())?;
    let mut known = compiled.lock().unwrap_or_else(|p| p.into_inner());
    if known.len() > 4096 {
        known.clear();
    }
    for (i, code) in missing.iter().zip(fresh) {
        known.insert(keys[*i], code.clone());
        out[*i] = Some(code);
    }
    Ok(out.into_iter().map(|c| c.expect("every script compiled")).collect())
}

fn run_compiler(sources: &[(&[u8], &str)]) -> Result<Vec<Vec<u8>>> {
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let luajit = find_compiler()?;
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("batch.lua"), BATCH)?;
    std::fs::write(dir.path().join("chunks.txt"), sources.iter().map(|(_, chunk)| format!("{chunk}\n")).collect::<String>())?;
    for (i, (source, _)) in sources.iter().enumerate() {
        std::fs::write(dir.path().join(format!("{i}.lua")), source)?;
    }
    let mut command = Command::new(&luajit);
    no_window(&mut command);
    let run = command.current_dir(dir.path()).arg("batch.lua").arg(sources.len().to_string()).output()?;
    if !run.status.success() {
        let raw = String::from_utf8_lossy(&run.stderr);
        let failed = raw.split_once('\n').and_then(|(i, message)| Some((sources.get(i.trim().parse::<usize>().ok()?)?.1, message)));
        let (chunk, message) = failed.unwrap_or(("luajit", &raw));
        return Err(ObsidianError::CompileFailed { chunk: chunk.to_string(), message: message.trim().to_string() });
    }
    (0..sources.len())
        .map(|i| {
            let code = std::fs::read(dir.path().join(format!("{i}.ljbc")))?;
            check_bytecode(&code)?;
            Ok(code)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_produces_2_0_bytecode() {
        let code = compile(b"x = 1", "t").unwrap();
        assert_eq!(&code[..5], &[0x1b, 0x4c, 0x4a, 0x01, 0x02]);
    }

    #[test]
    fn syntax_error_names_the_chunk() {
        match compile(b"x = ", "mods/a/main") {
            Err(ObsidianError::CompileFailed { chunk, message }) => {
                assert_eq!(chunk, "mods/a/main");
                assert!(message.contains("mods/a/main"), "{message}");
                assert!(!message.contains(".lua"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn same_bytes_as_the_compiler_flag() {
        let t = tempfile::tempdir().unwrap();
        let (src, out) = (t.path().join("a.lua"), t.path().join("a.ljbc"));
        let source = b"local t = { 1, 2 }\nfunction f(x)\n  return x + #t, 'text'\nend\nreturn f\n";
        std::fs::write(&src, source).unwrap();
        let luajit = find_compiler().unwrap();
        let mut flag = Command::new(&luajit);
        if cfg!(not(windows)) {
            flag.env("LUA_PATH", luajit.with_file_name("lua").join("?.lua"));
        }
        assert!(flag.arg("-b").arg("-s").arg(&src).arg(&out).status().unwrap().success());
        assert_eq!(compile(source, "t").unwrap(), std::fs::read(&out).unwrap());
    }

    #[test]
    fn a_byte_order_mark_still_compiles() {
        assert_eq!(compile(b"\xef\xbb\xbfx = 1", "t").unwrap(), compile(b"x = 1", "t").unwrap());
    }

    #[test]
    fn compile_all_keeps_the_order() {
        let sources: [(&[u8], &str); 3] = [(b"return 1", "a"), (b"return 2", "b"), (b"return 3", "c")];
        let all = compile_all(&sources).unwrap();
        for (i, (source, chunk)) in sources.iter().enumerate() {
            assert_eq!(all[i], compile(source, chunk).unwrap());
        }
        assert!(compile_all(&[]).unwrap().is_empty());
    }

    #[test]
    fn compile_all_names_the_chunk_that_failed() {
        match compile_all(&[(b"x = 1", "fine"), (b"x = ", "broken"), (b"y = 2", "unread")]) {
            Err(ObsidianError::CompileFailed { chunk, message }) => {
                assert_eq!(chunk, "broken");
                assert!(message.contains("broken"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
}
