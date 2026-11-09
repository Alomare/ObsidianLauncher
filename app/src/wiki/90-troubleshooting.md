---
title: When something goes wrong
group: Help
summary: Logs, crashes at startup, mods that don't load, and getting back to vanilla.
---

## Back to vanilla first

When the game misbehaves, **Purge** takes every Obsidian file out of the game. If the problem stays after a purge, Obsidian isn't the cause; verify the game files in Steam.

## The game crashed

The loader writes a line just before it starts each mod. If the game stops at that moment, the next time you open Obsidian it names the mod that was starting. Turn that mod off, deploy, and tell the author.

If the game stops later, Obsidian tells you the game didn't close normally. A crash inside the game's engine usually leaves no error in the log, so try turning off the mods you added last, one at a time.

## A mod does nothing

- Check that the mod is on and that you deployed after turning it on. A green **Deploy** button means changes are waiting.
- For a session mod, check the lobby: in a public lobby with vanilla players, or players who block session mods, session mods switch off ([Playing online](wiki:multiplayer)).
- Read the logs (below). A mod that failed at startup has an error line there.

## "Game files changed" {#build-unknown}

Some game files don't match the build Obsidian knows. This happens when a mod was copied straight into the game's `contents` folder over the original files. Deploy and Play still work, and clicking the warning lists the changed files. Purge, then verify the game files in Steam to put the originals back. Then add that mod in Obsidian with **Add** instead, which deploys it without touching the originals.

If the game itself says the version is unknown, the game program isn't the version Obsidian was made for, and no mod runs.

## Logs

The logs are in `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`:

| File | What it holds |
|---|---|
| `loader.log` | Each mod starting, hooks going in, the multiplayer state, and what each mod changed outside the loader |
| `<mod id>.log` | The lines that mod wrote with `obsidian.log`, and nothing else |

## Asking for help

Use **Copy report** in [Settings](app:settings), under Data. It copies your versions, the mods that are on and the end of `loader.log`, without your Windows account name. Paste it, with what happened, into the [Discord](https://dsc.gg/hd1modding) or a [GitHub issue](https://github.com/Alomare/ObsidianLauncher/issues).
