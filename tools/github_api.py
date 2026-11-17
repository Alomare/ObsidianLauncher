"""GitHub's REST API for the release tools, so they need no `gh`. The token is the one git pushes with, read
from Git Credential Manager for the account Alomare and never printed. A draft release is only visible with it.
"""
import json
import os
import subprocess
import urllib.request
from pathlib import Path

API = "https://api.github.com"
_token = None


def token():
    global _token
    if _token is None:
        env = {**os.environ, "GCM_INTERACTIVE": "never", "GIT_TERMINAL_PROMPT": "0"}
        out = subprocess.run(["git", "credential", "fill"], input="protocol=https\nhost=github.com\nusername=Alomare\n\n",
                             capture_output=True, text=True, env=env).stdout
        _token = next((l[len("password="):] for l in out.splitlines() if l.startswith("password=")), "")
        if not _token:
            raise SystemExit("no GitHub credential for Alomare in Git Credential Manager")
    return _token


def call(method, path, body=None, accept="application/vnd.github+json"):
    url = path if path.startswith("http") else API + path
    req = urllib.request.Request(url, method=method, data=json.dumps(body).encode() if body is not None else None,
                                 headers={"Authorization": f"Bearer {token()}", "Accept": accept,
                                          "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "obsidian-release"})
    with urllib.request.urlopen(req, timeout=300) as r:
        data = r.read()
    return json.loads(data) if accept.endswith("json") and data else data


def releases(repo, count=30):
    return call("GET", f"/repos/{repo}/releases?per_page={count}")


def release(repo, tag):
    """The release of a tag, drafts included (the tag lookup doesn't find drafts)."""
    found = next((r for r in releases(repo) if r["tag_name"] == tag), None)
    if not found:
        raise SystemExit(f"no release for {tag} in {repo}")
    return found


def download(rel, names, out):
    out = Path(out)
    for a in rel["assets"]:
        if a["name"] in names:
            (out / a["name"]).write_bytes(call("GET", a["url"], accept="application/octet-stream"))


def publish(repo, rel):
    return call("PATCH", f"/repos/{repo}/releases/{rel['id']}",
                # A prerelease never becomes latest: the updater reads through the latest link.
                {"tag_name": rel["tag_name"], "draft": False, "make_latest": "false" if rel["prerelease"] else "true"})
