"""Text for the Nexus Mods workflow (.github/workflows/nexus.yml).

    python tools/nexus_release.py changelog body.md   # the version's changelog, from the release text
    python tools/nexus_release.py files               # the current title and description of the Setup and Portable files

changelog: one line per manager change. A highlight becomes "Title: paragraph" on one line, a list entry its own line;
the section headings, list marks, bold and link targets go. Obsidian Kit updates (the Obsidian API, Trainer and QoL
highlights, and list entries that update one) stay out: the kit has its own pages.

files: the upload action always sends a title (the file name, when none is given), so the workflow passes back what
the page has now and the titles and descriptions set by hand on Nexus stay. Read from the public v2 API, no key
needed. The live (MAIN) file whose title says "Setup", and the one that says "Portable"; anything else stops the run
before an upload. Prints lines for $GITHUB_OUTPUT.
"""

import json
import re
import sys
import urllib.request

GAME_ID = 3552   # helldivers
MOD_ID = 16      # nexusmods.com/helldivers/mods/16
KIT = re.compile(r"^(Updated\s+)?Obsidian\s+(API|Trainer|QoL|Quality of Life)\b", re.I)


def changelog(body):
    out, title, para = [], None, []

    def close():
        nonlocal title, para
        if title and not KIT.match(title):
            out.append(f"{title}: {' '.join(para)}" if para else title)
        title, para = None, []

    for line in body.replace("\r", "").split("\n"):
        s = line.strip()
        if s.startswith("### "):
            close()
            continue
        if s.startswith("#### "):
            close()
            title = s[5:].strip()
            continue
        if not s:
            continue
        s = re.sub(r"\*\*(.+?)\*\*", r"\1", s)
        s = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", s)
        if title:
            if not re.match(r"^[-*]\s", s):  # a list under a highlight is detail: the paragraph carries it
                para.append(s)
            continue
        s = re.sub(r"^[-*]\s+", "", s)
        if not KIT.match(s):
            out.append(s)
    close()
    return "\n".join(out)


def files():
    q = {"query": f"{{ modFiles(modId: {MOD_ID}, gameId: {GAME_ID}) {{ name description category }} }}"}
    req = urllib.request.Request(
        "https://api.nexusmods.com/v2/graphql", data=json.dumps(q).encode(),
        headers={"Content-Type": "application/json", "User-Agent": "ObsidianModManager-release"},
    )
    with urllib.request.urlopen(req, timeout=30) as r:
        data = json.load(r)
    if data.get("errors"):
        sys.exit(f"Nexus refused the file list: {data['errors']}")
    live = [f for f in data["data"]["modFiles"] if f["category"] == "MAIN"]
    lines = []
    for key, word in (("setup", "setup"), ("portable", "portable")):
        hits = [f for f in live if word in f["name"].lower()]
        if len(hits) != 1:
            sys.exit(f"Expected one live Nexus file with \"{word}\" in its title, found {len(hits)}: "
                     f"{[f['name'] for f in live]}. Nothing uploaded.")
        f = hits[0]
        lines.append(f"{key}_name={f['name']}")
        desc = (f["description"] or "").replace("\r", "")
        lines.append(f"{key}_description<<DESC_END\n{desc}\nDESC_END")
    return "\n".join(lines)


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "changelog":
        print(changelog(open(sys.argv[2], encoding="utf-8").read()))
    elif len(sys.argv) == 2 and sys.argv[1] == "files":
        print(files())
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
