---
title: Obsidian API
group: Making mods
summary: A library for mods: game data by name, checked changes, effects, events and ready actions.
---

Obsidian API is a mod other mods build on. It finds things in the game by name, changes them in ways the game accepts, and undoes the changes when it has to. Obsidian Trainer is built on it and uses nothing that other mods can't.

This page is the overview. Every call, with arguments, answers and examples, is in [REFERENCE.md](https://github.com/Alomare/ObsidianAPI/blob/main/REFERENCE.md), which also comes inside the Obsidian API folder of the Obsidian Kit, where an AI tool working on a mod can read it.

## Using it

List it in `requires`, and Obsidian switches it on with the mod and starts it first:

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

Then ask for it in the script:

```lua
local api = obsidian.import("obsidian.api")

obsidian.keybind("panic", function()
  api.player.heal()
  api.enemies.kill_near(20)
end)
```

## Client mods and session mods

Obsidian API is a [library](wiki:session-mods#libraries), so it takes the scope of the mods that require it. A client mod can require it and stay a client mod, with no effect on matchmaking or the Galactic War. A client mod can read anything, use what only this player sees or hears (`api.ui`, `api.map`, `api.audio` and `api.music`) and listen to `api.events`. Every call that changes the shared game answers `false, "needs_session"` in a client mod.

In a session mod, those calls work while the lobby is cleared. When the lobby stops being cleared, Obsidian API takes the effects and edits of the mod off, and the calls answer `false, "not_cleared"`. When the lobby clears again, it puts them back. An expansion gets every call while the content runs, and `false, "not_started"` outside it ([Writing an expansion](wiki:session-mods#expansions)).

## How calls answer

Actions answer `true`, or `false` and a short reason such as `"host_only"`, `"no_avatar"` or `"not_in_mission"`. Calls that make something, such as `api.bots.add`, answer the new thing, or `nil` and a reason. An ordinary refusal never raises an error:

```lua
local ok, why = api.stratagems.call("stratagem_resupply", api.world.cursor())
if not ok then obsidian.toast("No resupply: " .. why) end
```

## What it holds

| Table | For |
|---|---|
| `api.game` | The current state, the local player, the host, and safe writes to guarded values |
| `api.lobby` | The players in the lobby, and short texts shared with every machine in it |
| `api.catalog` | Weapons, stratagems, perks, enemies, vehicles and pickups by name: the base game and the DLC the player owns |
| `api.edits` | Changes to settings tables, checked and recorded, with undo |
| `api.effects` | Changes that stay on across respawns and missions, such as god mode or speed, and custom effects |
| `api.events` | Calls when the game reports a kill, a death, an objective, a stratagem and more |
| `api.player` | Health, lives, respawn, teleport, looks and status effects |
| `api.gear` | Weapons, backpacks, perks, drones and refills, and the stats of one weapon |
| `api.vehicles` | The vehicle being driven, repairs and refills |
| `api.stratagems` | Cooldowns, uses, call-ins, and new stratagems made from ones in the game |
| `api.enemies` | Finding, spawning, damaging and removing enemies, and how hard encounters push |
| `api.mission` | Objectives, extraction, samples, proving ground modifiers, and ending a mission |
| `api.combat` | Damage multipliers for players and enemies |
| `api.world` | Positions, props, the camera, lights, time of day and the look of a mission |
| `api.map` | The mission map of this player: zoom, pings and markers |
| `api.audio`, `api.music` | Volumes, sounds and the soundtrack |
| `api.menu` | The in-game window that **F6** opens, where each mod has a folder of pages |
| `api.mod_settings` | A page of settings for the mod under **Mods** in the pause menu of the game ([Options in the pause menu](wiki:options-and-keys#options-in-the-pause-menu)) |
| `api.ui` | Hints, map pings, the menus of the ship, and game text shown with other words |
| `api.progress` | The save: samples, unlocks, upgrades and the loadout |
| `api.console` | The developer commands of the game, minus the ones that reach the shared backend |
| `api.bots` | Helldivers this machine plays, with orders, gear and help |
| `api.nav` | Walking routes for bots and mods |
| `api.capture` | Screenshots of the game, saved as image files |

## Rules worth knowing

The calls of Obsidian API already keep to these rules. They matter when a mod reaches past the API to the tables and functions of the game:

> [!WARNING]
> The game keeps hashed copies of guarded values such as `hitpoints_real`, `rounds`, `cooldown_timer`, `uses_left`, `num_grenades` and `extra_lives`. Writing one directly trips the anti-tamper check of the game, which cuts the player off from the network until the game restarts. Use `api.game.protected` or a queued command. `api.edits` refuses those keys.

- A queued command the game doesn't know, or a spawned unit whose package isn't loaded, stops the game with an error no script can catch.
- A timer the game runs while above zero, such as the `drop_time` of a hellpod, never finishes at 0. Use 0.01.
- The vectors of the game last only the frame they were made in. Copy the numbers to keep them.
- XP and rank reach the shared leaderboards with the next ordinary results, even after a purge.

REFERENCE.md has the full list.
