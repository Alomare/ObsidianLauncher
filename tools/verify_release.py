"""Checks a release on GitHub before it's published: the files the updater and players need, and that they
match what the tag holds.

Usage: python tools/verify_release.py v1.0.4 [--keep DIR]

Talks to GitHub through tools/github_api.py, with the token git pushes with. It downloads latest.json, the
signature and the portable ZIP of the release (a draft works) into a temporary folder, or into DIR with
--keep, and checks:

- the four files are there: the installer, its .sig, latest.json and the portable ZIP;
- latest.json has the tag's version, the notes of docs/releases/<version>.md, and for each platform the
  installer's own URL and the signature from the .sig file;
- the release text is the same notes;
- the version files at the tag all say the version;
- the portable ZIP holds the exe, portable.txt, the loader, the locales, LuaJIT and the Obsidian Kit, and
  each Kit mod is the latest release of its repository.

It ends with the command that walks the portable build. Exit code 0 when everything passed.
"""
import argparse
import json
import re
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

import github_api

ROOT = Path(__file__).resolve().parent.parent
REPO = "Alomare/ObsidianModManager"
KIT = {"obsidian.api": "Alomare/ObsidianAPI", "obsidian.trainer": "Alomare/ObsidianTrainer", "obsidian.qol": "Alomare/ObsidianPatches"}
VERSION_FILES = {
    "app/package.json": r'"version":\s*"([^"]+)"',
    "app/src-tauri/tauri.conf.json": r'"version":\s*"([^"]+)"',
    "app/src-tauri/Cargo.toml": r'(?m)^version\s*=\s*"([^"]+)"',
    "core/Cargo.toml": r'(?m)^version\s*=\s*"([^"]+)"',
}

failures = []


def check(ok, what, detail=""):
    print(("ok    " if ok else "FAIL  ") + what + (f": {detail}" if detail and not ok else ""))
    if not ok:
        failures.append(what)
    return ok


def git_show(tag, path):
    r = subprocess.run(["git", "show", f"{tag}:{path}"], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    return r.stdout if r.returncode == 0 else None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("tag", help="the release tag, such as v1.0.4")
    ap.add_argument("--keep", type=Path, help="download into this folder and keep the files")
    args = ap.parse_args()
    tag = args.tag
    version = tag.removeprefix("v")

    rel = github_api.release(REPO, tag)
    release = {"isDraft": rel["draft"], "isPrerelease": rel["prerelease"], "body": rel.get("body") or "",
               "assets": rel["assets"], "createdAt": rel["created_at"]}
    print(f"{tag}: {'draft' if release['isDraft'] else 'published'}{', prerelease' if release['isPrerelease'] else ''}")
    setup = f"Obsidian.Mod.Manager_{version}_x64-setup.exe"
    portable = f"ObsidianModManager-{version}-portable.zip"
    names = {a["name"] for a in release["assets"]}
    for name in (setup, setup + ".sig", "latest.json", portable):
        check(name in names, f"the release has {name}")
    if failures:
        raise SystemExit(1)

    notes_file = ROOT / "docs" / "releases" / f"{version}.md"
    notes = git_show(tag, f"docs/releases/{version}.md")
    check(notes is not None, f"the tag has docs/releases/{version}.md")
    notes = (notes or "").strip()
    check(release["body"].strip() == notes, "the release text is the notes file")
    if notes_file.is_file():
        check(notes_file.read_text(encoding="utf-8").strip() == notes, "the notes file on disk is the one at the tag")

    for path, pattern in VERSION_FILES.items():
        text = git_show(tag, path) or ""
        m = re.search(pattern, text)
        check(m is not None and m.group(1) == version, f"{path} at the tag says {version}", m.group(1) if m else "no version")

    tmp = None
    if args.keep:
        out = args.keep
        out.mkdir(parents=True, exist_ok=True)
    else:
        tmp = tempfile.TemporaryDirectory()
        out = Path(tmp.name)
    github_api.download(rel, {"latest.json", setup + ".sig", portable}, out)

    latest = json.loads((out / "latest.json").read_text(encoding="utf-8"))
    signature = (out / (setup + ".sig")).read_text(encoding="utf-8").strip()
    check(latest.get("version") == version, f"latest.json says {version}", str(latest.get("version")))
    check((latest.get("notes") or "").strip() == notes, "latest.json carries the notes")
    url = f"https://github.com/{REPO}/releases/download/{tag}/{setup}"
    platforms = latest.get("platforms") or {}
    check("windows-x86_64" in platforms, "latest.json has windows-x86_64")
    for name, p in platforms.items():
        check(p.get("url") == url, f"latest.json {name} points at the installer", str(p.get("url")))
        check((p.get("signature") or "").strip() == signature, f"latest.json {name} has the installer's signature")

    with zipfile.ZipFile(out / portable) as z:
        files = set(z.namelist())
        top = "ObsidianModManager/"
        for need in ("Obsidian.exe", "portable.txt", "loader/src/boot.lua", "locales/en.json"):
            check(top + need in files, f"the portable ZIP has {need}")
        check(any(f.startswith(top + "luajit/") for f in files), "the portable ZIP has LuaJIT")
        for mod, repo in KIT.items():
            path = f"{top}bundled/{mod}/obsidian.json"
            if not check(path in files, f"the portable ZIP has {mod}"):
                continue
            have = json.loads(z.read(path).decode("utf-8")).get("version")
            # The workflow takes each Kit mod's latest release when it builds, so compare with the latest
            # one published before this release was made, not with today's.
            kit = github_api.releases(repo)
            then = [r for r in kit if not r["draft"] and not r["prerelease"] and r["published_at"] and r["published_at"] <= release["createdAt"]]
            then.sort(key=lambda r: r["published_at"])
            want = then[-1]["tag_name"].removeprefix("v") if then else None
            check(have == want, f"{mod} {have} is the latest release of when this one was built", f"that was {want}")

    print()
    if failures:
        print(f"{len(failures)} failed.")
    else:
        print("Everything passed. Before publishing, walk the GitHub build: unzip the portable ZIP, delete")
        print("portable.txt from the copy (it would make the app use its own data folder), then run")
        print("  OBSIDIAN_EXE=<copy>/Obsidian.exe OBSIDIAN_WALK=<scratch folder> node internal/tools/walk/walk_exe.mjs")
        print(f"and then publish it with: python tools/publish_release.py {tag}")
    if tmp:
        tmp.cleanup()
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
