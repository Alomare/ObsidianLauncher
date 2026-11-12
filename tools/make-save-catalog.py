"""Builds core/assets/save-catalog.json from internal/research/save-catalog-dump.json, a dump taken
from the running game. Keeps what the save editor needs and nothing else.

Run from the repository root: python tools/make-save-catalog.py
"""

import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent
DUMP = ROOT / "internal" / "research" / "save-catalog-dump.json"
OUT = ROOT / "core" / "assets" / "save-catalog.json"
# The decompiled DlcSettings: which entitlement grants which item.
DLC_LUA = ROOT / "internal" / "research" / "lifted" / "lua" / "settings" / "dlc_settings.lua"


def as_list(v):
    # The game's empty tables come out of the dump as {} where a list would be.
    return v if isinstance(v, list) else []


def dlc_entitlements():
    """Item name to entitlement id, from DlcSettings. Each entry's four lists come before the line naming the entry."""
    out = {}
    lists = []
    for line in DLC_LUA.read_text(encoding="utf8").splitlines():
        m = re.match(r'\(\{\}\)\.(?:perks|primary_weapon|stratagems|customization) = \{(.*)\}', line)
        if m:
            lists.extend(re.findall(r'"([^"]+)"', m.group(1)))
            continue
        m = re.match(r'\(\{ id = "(ENT\d+)", steam_id = \d+ \}\)\.unlocks = \{\}', line)
        if m:
            for name in lists:
                out[name] = m.group(1)
            lists = []
    return out


def main():
    dump = json.loads(DUMP.read_text(encoding="utf8"))
    data, text = dump["data"], dump["text"]
    entitlements = dlc_entitlements()

    def entitlement(item):
        # Every paid item names the entitlement that grants the item; a gap here is a bug in the data.
        return {"entitlement": entitlements[item["name"]]} if item.get("_dlc") else {}

    def display(key, fallback=None):
        t = text.get(key) if key else None
        if t is None or t.startswith("LOC:"):
            return fallback if fallback is not None else (key or "")
        return t

    def upgrades(item, mask):
        out = []
        for i, u in enumerate(as_list(item.get("upgrades"))):
            name = u.get("upgrade_name") or ""
            out.append({"name": name, "display": display(name), "value": (1 << i) if mask else i + 1})
        return out

    def item_of(item, kind):
        mask = kind == "weapon"
        out = {
            "name": item["name"],
            "display": display(item["name"]),
            "description": display(item.get("description"), ""),
            "dlc": bool(item.get("_dlc")),
            "upgrade_kind": "bitmask" if mask else "level",
            "max_upgrade": int(item.get("_max_upgrade") or 0),
            "upgrades": upgrades(item, mask),
            **entitlement(item),
        }
        if item.get("group"):
            out["category"] = item["group"]
        elif item.get("stratagem_type"):
            out["category"] = item["stratagem_type"]
        return out

    def items(key, kind):
        return [item_of(i, kind) for i in as_list(data[key]) if not i.get("hide_in_armory")]

    def hidden(key):
        return [i["name"] for i in as_list(data[key]) if i.get("hide_in_armory")]

    ranks = []
    for r in as_list(data["ranks"]):
        unlocks = []
        for names in r.get("unlocks", {}).values():
            unlocks.extend(as_list(names))
        ranks.append({"level": r["level"], "required_xp": r["required_xp"], "title": display(r.get("title")), "unlocks": unlocks})

    customization = {}
    for slot in ("helmet", "body", "cape", "gender"):
        # Every entry stays, in the game's order: the save stores a 1-based index into this list.
        customization[slot] = [
            {"name": i.get("name", ""), "display": display(i.get("name")), "dlc": bool(i.get("_dlc")), **entitlement(i)}
            for i in as_list(data["customization"][slot])
        ]

    out = {
        "version": 1,
        "limits": data["limits"],
        "boss_influence_required": data["boss_influence_required"],
        "default_loadout": data["default_loadout"],
        "static_stratagems": as_list(data["static_stratagems"]),
        "ranks": ranks,
        "primary_weapons": items("primary_weapon", "weapon"),
        "sidearm_weapons": items("sidearm_weapon", "weapon"),
        "stratagems": items("stratagems", "stratagem"),
        "perks": items("perks", "perk"),
        "customization": customization,
        "hidden": hidden("primary_weapon") + hidden("sidearm_weapon"),
    }
    OUT.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":"), sort_keys=True) + "\n", encoding="utf8", newline="\n")
    print(f"wrote {OUT} ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
