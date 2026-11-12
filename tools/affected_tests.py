"""Prints the checks worth running for what changed, so a small change doesn't pay for every suite.

python tools/affected_tests.py [<git ref>] [--run]

With no ref it reads the uncommitted changes plus the commits `dev` has past `origin/dev`; with a ref
(`main`, `v1.4.0`, `HEAD~3`) it reads everything since that ref. `--run` runs the offline commands in
order and stops at the first failure; the exe walk areas are only ever printed, since the walk opens
a window and needs a fresh build.

The full run stays the check before a tag: `cargo test`, `tools\\test-loader.cmd`, `npm test` in
`app`, and the whole exe walk. The table below is the map from paths to checks; a path that no rule
knows falls back to the full suite of that part of the repository, so a new file is never skipped.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LUAJIT = Path("target") / "luajit" / ("luajit.exe" if sys.platform == "win32" else "luajit")

CORE = "cargo test -p obsidian-core --lib {}"
LOADER = f'"{LUAJIT}" loader/tests/run.lua {{}}'
ALL_LOADER = f'"{LUAJIT}" loader/tests/run.lua'

# Core source file (without .rs) -> walk areas (internal/tools/walk/walk_exe.mjs, WALK_ONLY=...).
CORE_WALK = {
    "deploy": ["game"], "compose": ["game"], "bundle": ["game"], "backup": ["game"], "gate": ["game"],
    "loader": ["game"], "luajit": ["game"], "overlay": ["game"],
    "library": ["library"], "import": ["library"], "sort": ["library"], "options": ["library"],
    "option_layout": ["library"], "folders": ["library"], "manifest": ["library"], "parts": ["library"],
    "modset": ["library"], "edit": ["library"], "images": ["library"],
    "profiles": ["profiles"], "profile_zip": ["profiles"],
    "updates": ["updates", "chrome"], "starter": ["kit"], "report": ["crash"],
}

# Loader source file (without .lua) -> test files (a filter for run.lua). Anything else: every loader test.
LOADER_TESTS = {
    "backend_block": ["test_backend_block"], "boot": ["test_boot"], "expansion": ["test_expansion"],
    "guardrails": ["test_guardrails", "test_session"], "menu": ["test_menu", "test_api_menu"],
    "pool": ["test_pool"], "save_data": ["test_saves"], "session": ["test_session"],
    "settings": ["test_settings"], "status": ["test_status"], "storage": ["test_storage"],
    "ui": ["test_ui"], "war": ["test_war"], "game_text": ["test_war"],
}

# App source prefix -> walk areas.
APP_WALK = [
    ("app/src/mods/", ["library"]), ("app/src/saves/", []), ("app/src/commander/", []),
    ("app/src/wiki", ["languages"]), ("app/src/i18n", ["languages"]),
    ("app/src/theme", ["theme", "sizes"]), ("app/src/styles.css", ["theme", "sizes"]),
    ("app/src/colors", ["theme"]), ("app/src/ProfilesDialog", ["profiles"]),
    ("app/src/Updates", ["updates", "chrome"]), ("app/src/updates", ["updates", "chrome"]),
    ("app/src/StatusBar", ["chrome"]), ("app/src/Tooltip", ["chrome"]), ("app/src/Crash", ["crash"]),
    ("app/src/report", ["crash"]), ("app/src/DeployProgress", ["game"]), ("app/src/SettingsPage", ["theme", "kit"]),
]


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8").stdout


def changed(ref):
    names = set()
    if ref:
        names.update(git("diff", "--name-only", ref).splitlines())
    else:
        names.update(git("diff", "--name-only", "HEAD").splitlines())
        names.update(git("diff", "--name-only", "origin/dev...HEAD").splitlines())
    names.update(git("ls-files", "--others", "--exclude-standard").splitlines())
    return sorted(n for n in names if n)


def plan(files):
    """(commands, walk areas, notes) for the changed files."""
    cmds, walk, notes = [], set(), []

    def add(cmd):
        if cmd not in cmds:
            cmds.append(cmd)

    ui_files, all_loader, full_core = [], False, False
    for f in files:
        p = Path(f)
        if f.startswith("core/src/commander/"):
            add(CORE.format("commander"))
            if p.name == "relay.rs":
                notes.append("relay.rs changed: also run ObsidianCommander `python tools/e2e.py` (the relay against a real war server)")
            notes.append("Commander changed: the COMMANDER tab walk is ObsidianCommander `tools/walk_tab.mjs`")
        elif f.startswith("core/src/saves/"):
            add(CORE.format("saves"))
            add(LOADER.format("test_saves"))
        elif f.startswith("core/src/") and p.suffix == ".rs":
            if p.stem in ("lib", "error", "paths", "testutil", "hash"):
                full_core = True
            elif p.stem == "main":
                add("cargo test -p obsidian-core --test cli")
            else:
                add(CORE.format(p.stem + "::"))
                walk.update(CORE_WALK.get(p.stem, []))
                if p.stem in ("loader", "settings", "luajit"):
                    add(LOADER.format("test_boot"))
                    add(LOADER.format("test_settings"))
        elif f.startswith("core/assets/"):
            add(CORE.format("saves"))
        elif f.startswith("core/tests/"):
            add(f"cargo test -p obsidian-core --test {p.stem}")
        elif f in ("Cargo.toml", "Cargo.lock", "core/Cargo.toml"):
            full_core = True
        elif f.startswith("app/src-tauri/"):
            add("cargo test -p obsidian-app")
        elif f.startswith("loader/src/"):
            tests = LOADER_TESTS.get(p.stem)
            if tests:
                for t in tests:
                    add(LOADER.format(t))
            else:
                all_loader = True
            add(CORE.format("loader::"))
        elif f.startswith("loader/tests/test_") and p.suffix == ".lua":
            add(LOADER.format(p.stem))
        elif f.startswith("loader/"):
            all_loader = True
        elif f.startswith("locales/"):
            add("cargo test -p obsidian-core --test locales")
            add("cd app && npx vitest run src/test/i18n.test.ts")
            walk.add("languages")
        elif f.startswith("app/src/") or f in ("app/package.json", "app/package-lock.json", "app/vite.config.ts", "app/tsconfig.json", "app/index.html"):
            ui_files.append(f)
            for prefix, areas in APP_WALK:
                if f.startswith(prefix):
                    walk.update(areas)
        elif f.startswith(("tools/", ".github/", "docs/", "internal/", "assets/")) or p.suffix == ".md":
            pass
        else:
            notes.append(f"{f}: no rule for this path; run the full suite")
            full_core = all_loader = True

    if full_core:
        cmds = [c for c in cmds if not c.startswith("cargo test -p obsidian-core")]
        cmds.insert(0, "cargo test -p obsidian-core")
    if all_loader:
        cmds = [c for c in cmds if "loader/tests/run.lua" not in c]
        cmds.append(ALL_LOADER)
    if ui_files:
        config = [f for f in ui_files if not f.startswith("app/src/")]
        if config:
            cmds.append("cd app && npm test")
        else:
            rel = " ".join(f[len("app/"):] for f in ui_files)
            cmds.append("cd app && npx tsc --noEmit && npx vitest related --run " + rel)
    return cmds, sorted(walk), notes


def main():
    args = [a for a in sys.argv[1:] if a != "--run"]
    files = changed(args[0] if args else None)
    if not files:
        print("Nothing changed.")
        return 0
    cmds, walk, notes = plan(files)
    print(f"{len(files)} changed files.")
    for c in cmds:
        print("  " + c)
    if walk:
        print(f"  exe walk (after a rebuild): WALK_ONLY={','.join(walk)} node internal/tools/walk/walk_exe.mjs")
    for n in dict.fromkeys(notes):
        print("  note: " + n)
    if not cmds and not walk:
        print("  No test covers these paths (docs, tools, workflows).")
    if "--run" in sys.argv:
        for c in cmds:
            print("\n> " + c, flush=True)
            if subprocess.run(c, cwd=ROOT, shell=True).returncode != 0:
                print("FAILED: " + c)
                return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
