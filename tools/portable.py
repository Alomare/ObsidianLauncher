"""Builds the portable ZIP from a release build: Obsidian.exe, what it ships beside it (the loader
sources, the bundled mods, the locales and the LuaJIT compiler) and portable.txt, which keeps the
data folder beside the app. Run after `npx tauri build` in app/.

Usage: python tools/portable.py   ->  target/release/bundle/portable/ObsidianLauncher-<version>-portable.zip
"""
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RELEASE = ROOT / "target" / "release"
version = json.loads((ROOT / "app" / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))["version"]

# The same files the installer carries (tauri.conf.json, bundle.resources), at the same places.
RESOURCES = [
    (ROOT / "loader" / "src", "loader/src"),
    (ROOT / "loader" / "fonts", "loader/fonts"),
    (ROOT / "bundled", "bundled"),
    (ROOT / "locales", "locales"),
    (ROOT / "target" / "luajit" / "luajit.exe", "luajit/luajit.exe"),
    (ROOT / "target" / "luajit" / "lua", "luajit/lua"),
    (ROOT / "THIRD-PARTY-NOTICES.txt", "THIRD-PARTY-NOTICES.txt"),
]

exe = RELEASE / "Obsidian.exe"
assert exe.is_file(), f"build the app first: {exe} is missing"
out = RELEASE / "bundle" / "portable" / f"ObsidianLauncher-{version}-portable.zip"
out.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    base = "ObsidianLauncher"
    z.write(exe, f"{base}/Obsidian.exe")
    z.writestr(f"{base}/portable.txt", "This copy of Obsidian is portable: the data folder sits beside the app.\n")
    for src, dest in RESOURCES:
        assert src.exists(), f"missing {src}"
        files = [src] if src.is_file() else sorted(p for p in src.rglob("*") if p.is_file())
        for f in files:
            rel = dest if src.is_file() else f"{dest}/{f.relative_to(src).as_posix()}"
            z.write(f, f"{base}/{rel}")
print(out)
