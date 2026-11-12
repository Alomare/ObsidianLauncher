---
title: What Commander is
group: Getting started
summary: Custom Galactic Wars hosted from Obsidian, and what runs on which PC.
wiki: commander
---

Commander runs custom Galactic Wars for HELLDIVERS. A custom war has fronts, regions, defenses, home world attacks and seasons like the official campaign, but the host sets every rule and number, and a Game Master can move fronts and start events from the war screen. Players fight the war in the game as usual, and the missions they finish move the custom war instead of the official one.

## What runs where

A war lives on one PC, the host. The war server starts from the [Commander tab](app:commander) and runs inside Obsidian, so a war only moves while Obsidian is open on that PC. When Obsidian closes, the server stops and the war clock freezes until the next start.

On every player PC, including the host, the game talks only to the Obsidian relay on that same PC. The relay carries the war requests to the war server and removes the Steam login ticket before anything leaves the PC. Obsidian has to stay open while a war is played, since the relay runs inside it.

Players who join a war from another PC need Obsidian and nothing else. The Commander package is for the host.

## Kept apart from the official war

Each custom war plays on a Sandbox save and in a separate matchmaking pool, so lobbies only pair players of the same war. The Play choice picks the save: any Sandbox, or a new one made for the war the first time it is selected with none picked. A host can keep a war to the Sandbox made for it. Steam achievements stay blocked on a Sandbox unless the host allows them for the war, and the achievements guardrail of the player still applies. Mission results go to the war server and nowhere else. The official Galactic War and the Official save never see a custom war, and for the same reason missions played with session mods count in a custom war ([The Galactic War](wiki:galactic-war) explains why they don't count in the official one).

## The package

The war server and the war screens come as the Commander package, which updates apart from Obsidian. **Install Package** on the Commander tab downloads the newest release from GitHub. Obsidian installs a downloaded package only with a valid signature from the Obsidian signing key and refuses any other.

The package button sits at the right end of the row under the war list, and a dot on it means a newer package is out:

| Item | What it does |
|---|---|
| Update Package | Downloads and installs the newest release |
| Install from File | Installs a package ZIP from disk, such as a local build. Obsidian doesn't check the signature of a file picked here, so only install files from a trusted source |
| Remove Package | Removes the war server and the war screens. Wars and war saves stay on the PC |
| Remove All Commander Data | Removes everything Commander keeps on the PC ([Removing Commander](wiki:commander-troubleshooting#removing-commander)) |
