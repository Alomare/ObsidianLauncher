"""Fills bundled/ with the Obsidian Kit, the mods that come with Obsidian: the release workflows do this
before they test and build, so the repository doesn't carry the mods. Takes each mod's latest release from
GitHub, the same zip Obsidian offers as an update. GITHUB_TOKEN, if set, avoids GitHub's limit for anonymous
requests.

Usage: python tools/starter_pack.py [--local] [--out FOLDER]

--local takes what the clones in ../Bundled have committed instead, to work on a mod and Obsidian together.
"""
import argparse
import io
import json
import os
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPOS = {"ObsidianAPI": "obsidian.api", "ObsidianTrainer": "obsidian.trainer", "ObsidianPatches": "obsidian.qol"}


def get(url, accept):
    request = urllib.request.Request(url, headers={"Accept": accept, "User-Agent": "obsidian-starter-pack"})
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def from_release(repo, mod, target):
    release = json.loads(get(f"https://api.github.com/repos/Alomare/{repo}/releases/latest", "application/vnd.github+json"))
    asset = next(a for a in release["assets"] if a["name"].startswith(mod + "-") and a["name"].endswith(".zip"))
    data = get(asset["browser_download_url"], "application/octet-stream")
    prefix = mod + "/"
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for name in z.namelist():
            assert name.startswith(prefix), f"{asset['name']}: {name} is outside {prefix}"
            relative = name[len(prefix):]
            if relative and not name.endswith("/"):
                out = target / relative
                out.parent.mkdir(parents=True, exist_ok=True)
                out.write_bytes(z.read(name))
    return release["tag_name"]


def from_clone(repo, target):
    src = ROOT.parent / "Bundled" / repo
    assert src.is_dir(), f"clone https://github.com/Alomare/{repo} into {src}"
    files = subprocess.run(["git", "-C", str(src), "archive", "--format=tar", "HEAD"], check=True, capture_output=True).stdout
    target.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(files)) as tar:
        tar.extractall(target, filter="data")
    return "local"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--local", action="store_true")
    parser.add_argument("--out", type=Path, default=ROOT / "bundled")
    args = parser.parse_args()
    for repo, mod in REPOS.items():
        target = args.out / mod
        shutil.rmtree(target, ignore_errors=True)
        version = from_clone(repo, target) if args.local else from_release(repo, mod, target)
        print(mod, version)


if __name__ == "__main__":
    main()
