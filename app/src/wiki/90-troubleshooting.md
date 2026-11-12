---
title: Troubleshooting
group: Help
summary: Logs, crashes at startup, mods that don't load, and getting back to vanilla.
pin: help
---

## Back to vanilla first

When the game misbehaves, **Purge** takes every Obsidian file out of the game. A problem that stays after a purge doesn't come from Obsidian, and verifying the game files in Steam is the next step.

## The game crashed

The loader writes a line just before it starts each mod. When the game stops at that moment, Obsidian names the mod that was starting the next time it opens. Turn that mod off, deploy, and tell the author.

When the game stops later, Obsidian only says that the game didn't close normally. A crash inside the engine usually leaves no error in the log, so turn off the mods added last, one at a time, until the crash goes away.

## A mod does nothing

- Check that the mod is on and that a deploy ran after turning it on. A green **Deploy** button means changes are waiting.
- For a session mod, check the lobby: in a public lobby with vanilla players, or with players who block session mods, session mods switch off ([Playing online](wiki:multiplayer)).
- Read the logs below. A mod that failed at startup leaves an error line there.

## "Game files changed" {#build-unknown}

Some game files don't match the build Obsidian knows. This happens when a mod was copied straight into the `contents` folder of the game over the original files. Deploy and Play still work, and clicking the warning lists the changed files. To get the originals back, purge, then verify the game files in Steam, and add that mod in Obsidian with **Add** instead, which deploys it without touching the originals. When the game itself says the version is unknown, the game program isn't the version Obsidian was made for, and no mod runs.

## Logs

The logs are in `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`:

| File | What it holds |
|---|---|
| `loader.log` | Each mod starting, hooks going in, the multiplayer state, custom war lines, and what each mod changed outside the loader |
| `<mod id>.log` | The lines that mod wrote with `obsidian.log`, and nothing else |

## Asking for help

**Copy report** in [Settings](app:settings), under Data, copies the versions, the mods that are on and the end of `loader.log`, without the Windows account name. Paste it with a description of what happened into the [Discord](https://dsc.gg/hd1modding) or a [GitHub issue](https://github.com/Alomare/ObsidianLauncher/issues). Problems with custom wars have a separate page in the Commander Wiki ([Troubleshooting](wiki:commander-troubleshooting)).
