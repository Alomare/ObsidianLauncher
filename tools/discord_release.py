"""Posts a release to Discord as one embed: the version, a link to the release, and the release's highlights.

Only the ### Highlights section goes in, each #### heading in bold with the first sentence of its paragraph; the
additions, changes and fixes stay on the release page, which the embed links to.

    python tools/discord_release.py v1.2.0             # posts to the webhook in DISCORD_WEBHOOK
    python tools/discord_release.py v1.2.0 --dry-run   # prints the payload instead

Reads the release through the gh CLI (GH_TOKEN in Actions). NEXUS_URL, when set, adds a Nexus Mods link.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request

REPO = os.environ.get("GITHUB_REPOSITORY", "Alomare/ObsidianModManager")
GOLD = 0xF0B429  # --gold in app/src/styles.css
ICON = f"https://raw.githubusercontent.com/{REPO}/main/app/src-tauri/icons/128x128.png"
LIMIT = 4096  # Discord's cap on an embed description


def release(tag):
    out = subprocess.run(
        ["gh", "release", "view", tag, "--repo", REPO, "--json", "name,tagName,url,body,publishedAt,isDraft,isPrerelease"],
        check=True, capture_output=True, text=True, encoding="utf-8",
    ).stdout
    return json.loads(out)


def summary(text, enough=120, most=320):
    """The paragraph's opening: sentences until it reads as enough, never past `most` once it has one, and never a
    sentence that only leads into a list ("Mods can now:")."""
    # A sentence ends at . ! or ? followed by a space and a capital (or bold), so "1.9.2" and "e.g. this" stay whole.
    out = ""
    for s in re.split(r"(?<=[.!?:])\s+(?=[A-Z*`])", text.strip()):
        if out and (len(out) >= enough or len(out) + len(s) > most or s.endswith(":")):
            break
        out = f"{out} {s}".strip()
    return out


def highlights(body):
    """[(title, first sentence)] from the ### Highlights section."""
    items, title, para, inside = [], None, [], False

    def close():
        if title:
            items.append((title, summary(" ".join(para))))

    for line in body.replace("\r", "").split("\n"):
        s = line.strip()
        if s.startswith("### "):
            if inside:
                break
            inside = s[4:].strip().lower() == "highlights"
            continue
        if not inside:
            continue
        if s.startswith("#### "):
            close()
            title, para = s[5:].strip(), []
        elif s and title and not re.match(r"^[-*]\s", s):
            para.append(s)
    if inside:
        close()
    return items


def payload(rel):
    version = rel["tagName"].lstrip("v")
    lines = [f"**{t}**\n{s}" for t, s in highlights(rel["body"])]
    links = f"[Full changelog]({rel['url']})"
    nexus = os.environ.get("NEXUS_URL", "").strip()
    if nexus:
        links += f" · [Nexus Mods]({nexus})"
    links += " · Obsidian updates itself from within the app."

    desc = ""
    for line in lines:
        if len(desc) + len(line) + len(links) + 4 > LIMIT:
            break
        desc += line + "\n\n"
    desc += links

    embed = {
        "title": rel.get("name") or f"Obsidian {version}",
        "url": rel["url"],
        "description": desc,
        "color": GOLD,
        "thumbnail": {"url": ICON},
        "footer": {"text": "Obsidian Mod Manager"},
    }
    if rel.get("publishedAt"):
        embed["timestamp"] = rel["publishedAt"]
    return {"username": "Obsidian", "avatar_url": ICON, "embeds": [embed], "allowed_mentions": {"parse": []}}


def post(url, data):
    req = urllib.request.Request(
        url + ("&" if "?" in url else "?") + "wait=true",
        data=json.dumps(data).encode("utf-8"),
        headers={"Content-Type": "application/json", "User-Agent": f"DiscordBot (https://github.com/{REPO}, 1.0)"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            print(f"Posted: HTTP {r.status}")
    except urllib.error.HTTPError as e:
        sys.exit(f"Discord refused the post: HTTP {e.code} {e.read().decode('utf-8', 'replace')}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("tag")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    rel = release(args.tag)
    if rel["isDraft"]:
        sys.exit(f"{args.tag} is a draft: not posted.")
    data = payload(rel)
    if args.dry_run:
        print(json.dumps(data, indent=2, ensure_ascii=False))
        return
    hook = os.environ.get("DISCORD_WEBHOOK", "").strip()
    if not hook:
        sys.exit("DISCORD_WEBHOOK is not set.")
    post(hook, data)


if __name__ == "__main__":
    main()
