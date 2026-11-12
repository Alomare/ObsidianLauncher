---
title: Obsidian API
group: Making mods
summary: A library for mods: game data by name, checked changes, and ready actions.
---

Obsidian API is a mod other mods build on. It knows where things are in the game, makes changes in ways the game accepts, and undoes them when it has to. Obsidian Trainer is built on it and uses nothing that other mods can't.

This page is the short version. The full guide, with every call, effect and recipe, is [REFERENCE.md](https://github.com/Alomare/ObsidianAPI/blob/main/REFERENCE.md), which also comes inside the API folder in the Obsidian Kit, where an AI tool working on a mod can read it.

## Using it

List it in `requires`, and Obsidian switches it on with the mod and starts it first:

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

Obsidian API is a [library](wiki:session-mods#libraries). A client mod can require it and stay a client mod, with no effect on matchmaking or the Galactic War. In that case it can read game data and use `api.ui`, `api.audio` and `api.events`, and anything that changes the shared game answers `false, "needs_session"`. To use those, make the mod a session mod, or an expansion, which gets all of Obsidian API while the content runs and `false, "not_started"` outside it ([Writing an expansion](wiki:session-mods#expansions)). An expansion leaves Obsidian API a client mod at deploy.

Then ask for it:

```lua
local api = obsidian.import("obsidian.api")

obsidian.keybind("panic", function()
  api.player.heal()
  api.enemies.kill_near(20)
end)
```

Actions answer `true`, or `false` and a short reason such as `"host_only"`, `"no_avatar"` or `"not_in_mission"`.

## Changing game data {#edits}

`api.edits` changes settings tables and keeps a record of what changed, who changed it and what it was before.

```lua
-- Double the damage of the assault rifle, upgrades included.
local ok, why = api.edits.set_unit("equipment/wep_assaultrifle_01", "weapon_info.power", 150)

-- A value in any table the game reads.
local s = api.catalog.stratagem("stratagem_resupply")
api.edits.set_table("Resupply", s, "cooldown_time", 30)
```

- Types and ranges are checked, and `opts` can give `min` and `max`.
- Tables stay the same tables, because live units hold references to them.
- A weapon has a copy of the settings for each upgrade, and an edit reaches them all.
- `api.edits.list()`, `revert(id)` and `revert_all()` show and undo changes.
- `opts.also` in `set_table` lists more paths that take the same value, as one change.
- When the lobby stops being cleared, every edit is undone. When it clears again, they return.

The upgrade levels of a stratagem can set a field for themselves, so a change to the base stops at those levels. `api.stratagems.edit` changes every level at once, with the ranges of the field:

```lua
api.stratagems.edit("stratagem_railcannon_strike", "cooldown_time", 30)
```

Fields a player can edit, with ranges and when each takes effect, are in `api.catalog.fields(family)`. `api.catalog.families_of(settings)` says which families a settings table has. Some numbers apply at once and some at the next spawn. `api.gear.refresh()` pushes the cached ones into weapons in play.

## Effects {#effects}

An effect stays on across respawns and missions. The game rebuilds much of the player state then, so the library applies active effects again.

```lua
api.effects.set("god", true)
api.effects.set("speed", 2)     -- a value, false to switch off
api.effects.is_on("god")
```

A custom effect:

```lua
api.effects.define({
  id = "mymod.low_gravity",
  apply = function(ctx, value) end,            -- on, and again after a respawn or new mission
  frame = function(ctx, value, dt) end,        -- every frame while on
  remove = function(ctx, value) end,           -- off
})
```

`ctx` has `state`, `name`, `record` and `unit`. All of them can be nil. Give every effect a `remove`, so it can come off when the lobby stops being cleared.

## Events {#events}

`api.events.on(name, fn)` calls `fn` when the game says something happened, so the mod doesn't have to look every frame.

```lua
local kills = 0

api.events.on("on_enemy_killed", function(unit, attacker, owner)
  kills = kills + 1
  if kills == 50 then api.ui.hint("Fifty down") end
end)
```

| Event | `fn` gets | When |
|---|---|---|
| `on_enemy_killed` | `unit`, `attacker`, `owner`, `damage_type` | This player got a kill. `damage_type` is there only when this game worked out the hit |
| `on_enemy_spawned` | `unit`, `enemy_name` | An enemy appeared, in the game of every player |
| `on_player_death` | `unit` | A player died, in the game of every player |
| `on_objective_completed` | nothing | An objective was finished |
| `on_stratagem_activated` | `peer`, `stratagem`, `position`, `unit` | The countdown of a stratagem ended, in the game of every player |
| `on_avatar_extract` | `unit`, `shuttle` | This player boarded the shuttle |
| `on_killed` | `unit`, `attacker`, `owner` | A unit with health was killed |
| `on_dead` | `unit` | The death of a unit finished |
| `on_health_downed` | `unit`, `state` | A player went down |
| `on_damage_taken` | `unit`, `hit` | A unit lost health. `hit` has `damage`, `hit_zone`, `attacker_unit` and `owner_unit` |
| `on_health_received` | `unit`, `amount` | A unit was healed. A full heal passes neither |
| `on_stratagem_pre_activated` | `stratagem`, `position`, `owner`, `flare` | The countdown of a flare started |
| `on_stratagem_used` | `unit`, `data` | A player used a stratagem |
| `on_stratagem_wielded` | `unit`, `flare`, `info` | A player took a flare in hand |
| `on_extract_shuttle_landed` | `unit` | The shuttle landed |
| `on_add_to_stat` | `unit`, `stat`, `amount` | A statistic went up, such as `"stratagems_used"` or `"hits"`. This one comes often |
| `on_pause` | `paused` | The pause menu opened or closed |
| `on_mission_end` | `stats`, `extra` | The end screens have the results |

- A function that fails counts against the mod that added it, and the others still run.
- Nothing arrives while the lobby isn't cleared.
- `position` and `hit` last one frame. Copy what you need from them.
- `api.events.off(name, fn)` stops one.

## What's in the API {#contents}

| Table | For |
|---|---|
| `api.game` | The current state, the local player, `is_host()`, and safe writes |
| `api.catalog` | Weapons, stratagems, perks, enemies and groups by name: the base game, plus the DLC the player owns. `usable(name)` checks that, `unowned_path(path)` checks the path of a weapon, and `unlocked(name)` says whether the player has unlocked an item by playing. Also `backpacks`, `crates`, `pickups`, `vehicles`, `by_subtype(name)` and `max_upgrade(path)` |
| `api.player` | `heal`, `heal_by(amount)`, `set_health(n)`, `respawn`, `teleport`, `set_cosmetic`, `set_outline(r, g, b)`, `clear_status(name)`, `immune(name, on)`, `knockdown`, `drop_carried`, and more |
| `api.gear` | `refill`, `give_weapon(slot, path, upgrade, unit)`, `give_backpack(path, upgrade, unit)`, `give_perk(name, unit)`, `remove_perk(unit)`, `add_perk(name, unit)`, `remove_extra_perk(name, unit)`, `extra_perks(unit)`, `refresh`, `weapons_of(owner)`, `tune(unit, component, changes)`, `add_drone(path, unit, upgrade)` |
| `api.stratagems` | `reset_cooldowns`, `refill_uses`, `give`, `drop`, `call(name, pos, delay)`, `edit(name, field, value)`, `define(def)`, `custom()` |
| `api.enemies` | `spawn`, `spawn_group`, `spawn_entity(path, pos)`, `kill_all`, `kill(unit)`, `list`, `info(unit)`, `nearest(pos)`, `apply_status(unit, template)`, `stun_near(radius)`, `near(radius, pos)`, `damage(unit, power, statuses)`, `set_hitpoints(unit, n)`, `despawn(unit)`, `load_faction(name)`, `set_pressure(t)`, `waves()`, `force_wave(name, pos)`, `carried()`, the enemies on the map with corpses and the spawn queue, and `room()`, how many more fit under `MAX_SAFE` |
| `api.mission` | `finish_objectives`, `objectives()`, `mutator(name)`, `win`, `abort`, `practice`, `drop` |
| `api.vehicles` | `current()`, `driving()`, `repair()`, `refill()` |
| `api.combat` | `scale{ players, enemies }` |
| `api.events` | `on(name, fn)`, `off(name, fn)` |
| `api.ui` | `hint(text, seconds)`, `ping(pos)`, `open(name, opts)`, `close()`, `menus()`, `with_texts(texts, fn, ...)` |
| `api.menu` | The in-game window (F6): `add(title, build, opts)`, `open()`, `close()`, `toggle()`, `is_open()`, `toast(text, seconds)`. The pages of each mod sit in a folder named after the mod |
| `api.map` | `is_open(unit)`, `open()`, `close()`, `zoom()`, `set_zoom(z)`, `view()`, `cursor()`, `look_at(pos)`, `pings()`, `to_map(pos)`, `to_world(x, y)`, `marker(pos, opts)`, `clear_markers()` |
| `api.bots` | `add(opts)`, `all()`, `remove_all()`, and on a bot the orders, gear and help |
| `api.lobby` | `players()`, `share(name, value)`, `shared(name)` |
| `api.world` | `cursor`, `ahead`, `to_screen(pos)`, `prop(unit, pos, opts)`, `explode`, `camera`, `shading(name, kind, value)`, `flash(seconds)` |
| `api.audio` | `bus(name, db)`, `play(event, pos)`, `stinger(name)` |
| `api.music` | `now()`, `tracks()`, `play(track)`, `stop()`, `release()`, `pause()`, `resume()`, `volume(db)` |
| `api.progress` | `set_samples`, `unlock_items`, `upgrade_all`, `loadout()`, `set_loadout(t)`, `upgrades()`, `set_upgrade(name, value)`, `upgrade_limit(name)` |
| `api.console` | `run(line)`, `commands()` |

## Player {#player}

These effects stay on across respawns. Most give back what was there before, so the original value of a perk returns when they come off.

| Effect | Value |
|---|---|
| `armor` | Armor at every angle |
| `sprint_speed` | Top sprint speed |
| `throw_range` | A multiplier on throws |
| `acceleration`, `deceleration` | How fast the player starts and stops |
| `ignore_slow` | `true`: the slow multiplier of the mover doesn't apply, so nothing that works through it slows the player |
| `no_screen_leash` | `true`: the camera stops holding the player inside the screen |
| `hide_model` | `true`: the player model is hidden |
| `heavy_armor_look` | `true`: the look of the perk, without the armor |

- `api.player.set_outline(r, g, b)` changes the outline the player shows through walls, and `set_outline(nil)` gives it back.
- `api.player.clear_status(name)` takes off a status template, or every status of an element such as `"fire"`. `api.player.immune(name, on)` keeps one off across respawns, until `on` is `false`.
- `set_cosmetic("gender", index)` changes the character, with an index from `api.catalog.cosmetics("gender")`.

## Gear {#gear}

`upgrade` in `give_weapon` and `give_backpack` runs from 0 to 15. Leave it out for what the player owns for that item. `api.catalog.max_upgrade(path)` gives the highest one.

`give_backpack` takes only paths from `api.catalog.backpacks()`, because the game reads settings other paths don't have, where nothing can catch the error. The old pack drops to the ground, as the old weapon does.

`add_perk(name, unit)` puts a perk on a player on top of the main one, and `remove_extra_perk(name, unit)` and `extra_perks(unit)` go with it. A new body after a death starts with none. A perk with per-frame code, such as laser sight, answers `"not_available"`, because the game runs that code for the main perk alone.

`give_perk` changes perks the way the game console does. Laser sight answers `"alone_only"` with other players in the lobby, because the games of those players would run it with no setup.

### Stats of one weapon {#one-weapon-s-stats}

`api.data` edits change a weapon type for everyone on that machine. `api.gear.weapons_of(owner)` lists the weapons a player holds, as `{ unit, component, slot, info }`, and `api.gear.tune(unit, component, changes)` changes one of them alone. `changes` holds a factor for a number (`firerate = 1.2`) and `true` or `false` for an attribute (`armorpiercing = true`). Each call replaces the last, and `{}` takes it all off. It reaches only the weapons this machine owns, so the game of each player tunes the weapons of that player.

The component `"melee"` tunes the melee attack of the same weapon instead: `power`, `duration` (how long the swing holds the player, so a smaller one swings faster) and `delay` (how soon it hits).

```lua
for _, w in ipairs(api.gear.weapons_of()) do
  api.gear.tune(w.unit, w.component, { firerate = 1.2 })
end
```

### Drones without a backpack

`api.gear.add_drone(path, unit, upgrade)` gives a player the drones of a drone pack and leaves the backpack slot alone. `path` is a drone pack from `api.catalog.backpacks()`, such as `equipment/equipment_drone_defensive_backpack_01` or `equipment/equipment_drone_heal_backpack_01`. The drones need the station of the pack to dock, reload and fly around, so the station rides on the player as a worn pack does, and every game running Obsidian API hides it. Call it again for more drones. It answers a drone with `:alive()` and `:remove()`. The drones dock while the player sits in a vehicle, and go when the player dies, as a worn pack drops, so give them again after a respawn.

```lua
local dog = api.gear.add_drone("equipment/equipment_drone_defensive_backpack_01")
```

## Stratagems {#stratagems}

`api.stratagems.call` puts down a real flare, as the game does for timed drops. It has the sound and the countdown, patrols hear it, and it works for Reinforce. It spends no cooldown or use, and it's called at the level the player owns. A fourth argument, a unit, calls it for another player on this machine, who gets the call-in and the kills.

```lua
api.stratagems.call("stratagem_resupply", api.world.cursor(), 15)   -- 15 s on top of the usual call-in
```

`api.stratagems.drop` is the quick one: no flare and no wait, and no Reinforce.

The effect `ignore_jammers` lets stratagems work inside no-stratagem zones.

### Custom stratagems

`api.stratagems.define(def)` adds a stratagem to the game list, made from a stratagem in the game, the base. It answers `{ name, id }`, or `nil` and a reason. Make it as the mod starts, so a saved loadout that has it still finds it.

```lua
api.stratagems.define({
  name = "mymod_pitchfork",                 -- new, letters, digits and _
  base = "stratagem_lasercannon",           -- copied, and what other players see
  title = { en = "Pitchfork", fr = "Fourche" },
  description = "Three barrels.",
  code = { "down", "left", "up", "down", "left", "right" },
  icon = "stratagem_lasercannon",           -- any icon the HUD has
  type = "supply",                          -- supply, offensive, defensive or special
  stats = { cooldown_time = 60, num_uses = 2, activation_time = 2 },
  upgrades = {
    { title = "Pitchfork Light", description = "Move while firing.", cost = 3, stats = { cooldown_time = 45 } },
  },
  on_activate = function(ctx)               -- on the machine of the thrower, as it's called down
    api.world.explode(ctx.position, 2000, 8)
    return false                            -- false: drop nothing else
  end,
})
```

- Every field but `name`, `base` and `title` is optional, and keeps the value of the base. Without `upgrades`, the levels of the base stay, named after the new stratagem: "Pitchfork Mk2".
- `unit` sets what lands, and `subtypes` and `attributes` what the Loadout screen shows. Anything it calls down has to be a unit every game has.
- `ctx` has `name`, `position` (a game vector, good for that frame), `owner` (the unit of the thrower) and `upgrade`.
- Other players always see and hear the base. The ids in game messages, and in the fields the game copies to other machines, go out as the base, so a player without the mod is never sent one the game doesn't have. That holds in any lobby. In a lobby that isn't cleared, the stratagem is the base, with the numbers and call-in of the base and no `on_activate`.
- The Loadout screen offers it to a player who has the base, by rank, reward or an owned DLC, and only in a cleared session.
- The save keeps the upgrades under the stratagem name. A saved loadout that names a stratagem the game doesn't have gets the default instead, so a player can take the mod off and keep a save that works.
- `api.stratagems.custom()` lists the ones made so far, as `{ name, base, id }`.

## Enemies {#enemies}

```lua
local bug, metres = api.enemies.nearest()
if bug then api.enemies.apply_status(bug, "flashbang_stun") end

-- Bigger fights on the same mission. false gives a number back to the game.
api.enemies.set_pressure({ cap = 24, difficulty = 15, patrols = 20, interval = 4 })
api.enemies.set_pressure({ difficulty = false })
```

- `apply_status` works on any enemy from the game of any player. Status names are in `api.catalog.status_templates()`.
- `info(unit)` gives `key`, `name`, `faction`, `boss`, `hitpoints`, `max_hitpoints` and `owned`.
- `set_pressure` works for the host only. `cap` runs up to 32 and lifts both limits waves stop at. Corpses count.
- `difficulty` is what each encounter can spend. The seed, the waves on offer and the objectives stay the same.
- `force_wave` spawns a wave from `api.enemies.waves()` all at once, ignoring the wave delays.
- `spawn(key, count, where)` takes `where` as `"cursor"`, a position (a vector or `{ x, y, z }`), or nothing for in front of the player.
- `spawn_entity` spawns a crate or pickup. Take paths from `api.catalog.crates()` and `pickups()`, which leave out paid items.
- `near(radius, pos)` lists living enemies, nearest first, around a position or the player.
- `damage(unit, power, statuses)` is a hit from this player that works from the game of any player, and armor counts. `statuses` is an optional list of status names.
- `set_hitpoints(unit, n)` and `despawn(unit)` go through the game of whichever player owns the enemy.
- `load_faction(name)` loads `"bugs"`, `"cyborgs"` or `"illuminates"` during a mission, so any of them can spawn on any planet. It answers `"alone_only"` with anyone else in the lobby, because every game needs the package. The game stops for a moment while it loads, and it's unloaded when the mission ends.

## Missions {#missions}

`api.mission.objectives()` lists `name`, `text`, `type`, `state`, `group`, `done`, `total` and `position`. `state` is `pending`, `complete` or `failed`.

The effect `protect_objectives` keeps convoys and escorted people alive. It reaches what the game of this player runs, so use it on the host.

`api.mission.mutator(name)` turns on one of the proving ground modifiers of the game in any mission, and `nil` turns it off. There's one at a time, and a real proving ground keeps the modifier it has.

| Name | What it does |
|---|---|
| `super_boots` | 40% faster on foot |
| `what_goes_around` | Damage to a teammate comes back to the shooter |
| `orbital_support` | Support stratagems land near a player every 20 s |
| `orbital_bombardments` | Strikes land near a player every 15 s, plus their call-in |

## Vehicles {#vehicles}

These effects reach the vehicle the player is driving, and let go of it when the player gets out. The game of a passenger can't change it.

| Effect | Value |
|---|---|
| `vehicle_speed` | A multiplier |
| `vehicle_turn` | A multiplier |
| `vehicle_armor` | Armor at every angle |
| `vehicle_god` | `true`: no damage |
| `vehicle_rockets` | `true`: exosuit rockets never run out |

The fields of the `vehicle` family change a vehicle for good. Top speed takes effect when the driver gets out and back in.

## Damage {#damage}

```lua
api.combat.scale({ players = 0.5, enemies = 2 })
```

A multiplier from 0 to 100, after armor, so armor that stops a shot still stops it. 1 or `false` turns a side off. The game passes no attacker to the damage sums, so a scale goes by who takes the hit. The game of each player works out the hits on that machine, so everyone in a lobby should use the same numbers.

## Camera and look {#view}

All of this stays with this player: nobody else sees it. Obsidian API is a session mod, so it stops while the lobby isn't cleared.

```lua
api.world.camera.fov(90)                    -- degrees, nil gives it back
api.world.camera.spin(30)                   -- degrees around the player
api.world.camera.spectate(friend_unit)      -- nil comes back
api.world.camera.photo(true)                -- freezes the camera and clears the screen
api.world.camera.place(position, rotation)  -- moves it, in photo mode

api.world.shading("fog_color", "vector3", { 0.5, 0.45, 0.3 })
api.world.shading("fog_color", "vector3")   -- back to the planet value
api.world.flash()                           -- lightning
```

- A low spin or angle can make aiming feel odd: the mouse aims in screen space.
- `spectate` takes the unit of another player, from `api.game.all_avatars()`.
- `shading` takes the names in `api.world.SHADING`, by kind: `scalar` is a number, `vector2` and `vector3` are lists. A value goes on again in each new mission. The game only ships code for `fog_color`, `sun_color` and the other `vector3` names, without using them, so check them on a first run.
- `flash` needs a forest planet, and not one of the illuminates. Others answer `"wrong_environment"`.

## Sharing between machines {#lobby}

`api.lobby.share(name, value)` puts a text of up to 200 characters where every other machine in the lobby can read it, under the id of the mod. `api.lobby.shared(name)` answers what each other machine shares, by peer. Steam passes a change on within about a second, so it suits state that changes now and then, such as a score, not every frame. Without a lobby, `share` answers `false, "not_available"`.

```lua
api.lobby.share("score", tostring(my_score))
for peer, text in pairs(api.lobby.shared("score")) do total = total + tonumber(text) end
```

`api.world.to_screen(pos)` answers where a point of the level is on the screen, in pixels from the bottom left, and whether it's in view, for drawing over the world with a kit.

## Props {#props}

`api.world.prop(unit, pos, opts)` puts a unit of the game in the level on this machine only. Nothing goes over the network and no game system knows about it, so it does nothing but be seen, with whatever glow and highlight the unit has. `opts.scale` sets the size, 1 as made. It answers a prop with `move(pos)`, `scale(size)`, `flow(event)`, `alive()` and `remove()`, or `nil` and a reason. `flow` runs one of the flow events of the unit, the way the game makes a unit play sounds and effects: `on_picked_up` on a research sample plays the pickup.

```lua
local sample = api.world.prop("gameobjects_spec/gameobject_bug_research_object", pos, { scale = 2 })
sample:move(new_pos)
sample:remove()
```

The unit has to be in a package the level has loaded: the game stops for one it can't find. The research samples of a mission, `gameobjects/flare_01` and the enemies of a loaded faction are safe.

## Sound {#sound}

Sound is local too.

```lua
api.audio.bus("combat_enemy", -30)   -- decibels; 0 is as mixed, -60 is silent
api.audio.bus("combat_enemy", false) -- back to the game mix

api.audio.play("ui_select")          -- anywhere
api.audio.play("ui_select", position)
api.audio.stinger("music_objective_found")
```

- `api.audio.BUSES` lists the buses. A volume repeats every frame, because the mix states of the game write them as they blend.
- `play` and `stinger` answer `"unknown_event"` for a sound this mission hasn't loaded.
- `stinger` goes through the stinger cooldown of the game, 5 seconds.

### The soundtrack

`api.music` picks what plays. A track you play holds on this screen and the next ones, and loops, until `release()` gives the music back to the game.

```lua
api.music.tracks()              -- the music events this screen has
api.music.play("music_credits") -- in place of the game music
api.music.stop()                -- silence, held the same way
api.music.pause()
api.music.resume()
api.music.release()             -- the game picks again
api.music.volume(-12)           -- decibels, as api.audio.bus("music_master", ...)
api.music.now()                 -- { place, track, held, paused }
```

- `api.music.TRACKS` lists every music event the game scripts start. A screen has only some of them, so `play` answers `"unknown_event"` for one it hasn't loaded, and `"no_music"` where there's no music at all. A held track that the next screen hasn't loaded leaves that screen silent.
- On the main menu and the ship the music plays in a separate sound world, so a pause stops it where it is. In a mission and on the start and end screens of the mission, a pause mutes the track, which keeps playing silently.
- In a mission, the threat and combat music of the game go quiet while a track is held, and come back when it's released.

## Loadouts {#loadouts}

```lua
api.progress.set_loadout({ primary_weapon = "weapon_railgun", stratagems = { [1] = "stratagem_resupply" } })
api.progress.set_upgrade("weapon_railgun", 3)
```

Both write the real save, on the ship, with the ship menus closed, and answer `"close_screen"` otherwise.

- `set_loadout` takes any of `perk`, `primary_weapon`, `sidearm_weapon` and `stratagems`, which maps the four free slots to names. It's applied at the next spawn, and it's put in the first preset as well, because the Loadout screen starts from that.
- Paid items and unknown names go back to the default, as when the game reads a save. An item the player hasn't unlocked doesn't survive the Loadout screen.
- A value in `set_upgrade` is what the game keeps: a bit per upgrade for weapons, a level for stratagems. `upgrade_limit(name)` gives the highest one.

## Messages {#messages}

`api.ui.hint(text, seconds)` shows the hint box of the game, even with hints turned off. `text` is the words to show, or a hint name such as `"hint_alarm"`. Words go through [`obsidian.notify`](wiki:loader-api#obsidian-notify-text-seconds), so it can answer `false, "queue_full"`. `api.ui.ping(pos)` puts the mark of this player on the map at a position, and `api.ui.ping(nil)` takes it away. Every player sees it.

`api.ui.open(name, opts)` opens a menu of the ship by name, as a terminal does, and `api.ui.close()` closes the top one. `api.ui.menus()` lists the names. `opts.seed` gives Loadout and the briefing a mission, and `opts.pad` opens it for a pad other than the one of the main player.

`api.ui.with_texts(texts, fn, ...)` runs `fn` with some game text keys showing other words: `texts` is `{ key = words }`. It only lasts while `fn` runs, so a mod can wrap a game screen and reuse it with new text, while every other text stays as the game has it.

```lua
api.ui.with_texts({ menu_campaign = "SURVIVORS" }, original, self, ...)
```

`api.lobby.players()` lists `peer`, `name`, `ping` in milliseconds, `host` and `me`. It only reads. A teammate has a name during a mission.

## The map {#map}

`api.map` reads and steers the mission map of the local player. All of it changes only what this player sees, so a client mod can use it.

- `is_open(unit)` says whether a map is open, the one of the local player when `unit` is nil. `open()` and `close()` do what the map key does. A map opened this way stays open until the player presses the map key and lets go, does something else such as a dodge or a reload, or `close()` is called.
- `zoom()` and `set_zoom(z)` read and set the zoom, from 0.5 to 4. `view()` is the world point under the crosshair, where a ping would land, and `cursor()` is another name for it. `look_at(pos)` pans so a point is under the crosshair.
- `pings()` lists the ping of every player as `{ record, owned, position }`. `api.ui.ping(pos)` sets the ping of the local player.
- `to_map(pos)` and `to_world(x, y)` convert between world positions and map units at zoom 1.

`marker(pos, opts)` puts a marker from the mod on the map of this player, and it goes when the mission ends. `opts` takes `icon` (a material the map has loaded, `icon_map_pickup_generic` by default), `size`, `color` as `{ a, r, g, b }`, and `unit` to follow a unit instead of staying at `pos`.

```lua
local m = api.map.marker(pos, { icon = "icon_map_objective_bunker_01", size = 32, color = { 255, 40, 255, 40 } })
m:move(new_pos)
m:remove()
```

## Bots {#bots}

`api.bots.add(opts)` makes a Helldiver this machine plays. It needs the host or a player alone, in a mission, and answers `nil, "host_only"` or `nil, "not_in_mission"` otherwise. `opts` takes `name`, `bare` and `position`.

A bot is first a local player on a pad nobody holds, so it has a HUD slot, a name, revives and reinforcements. Other players see an ordinary local co-op player. With the four player records taken, a bot is a bare Helldiver: a unit with the name over the head and no HUD slot or respawn. `bare = true` asks for one, and `bare = false` answers `nil, "no_slot"` instead. A bare bot needs a game pad number nobody uses to be driven, and stands still when none is left.

```lua
local bot = api.bots.add({ name = "Dummy" })
bot:follow(nil, 6)   -- keep within 6 metres of the player
bot:fight(25)        -- aim at and shoot the nearest enemy within 25 metres
bot:go_to(api.world.cursor())  -- walk there and stop
bot:hold()           -- stay put
bot:aim_at(pos)      -- keep facing a spot while there's nothing to fight
```

Orders: `follow(unit, distance)`, `go_to(pos, distance)`, `hold()`, `aim_at(pos)`, `fight(radius)`, and `fighting()` and `firing()` to read them back. A new order to walk replaces the last one, and `go_to` ends when the bot gets there.

Gear and help go to the bot as `api.gear`, `api.stratagems` and `api.player` give them to the main player, and answer `true` or `false` and a reason, `"no_avatar"` while the bot has no body: `give_weapon(slot, path, upgrade)`, `give_backpack(path, upgrade)`, `give_perk(name)`, `remove_perk()`, `refill()`, `give_stratagem(name)`, `call(name, pos, delay)` for a real call-in by the bot, `heal()`, `teleport(pos)`, and `weapons()`.

`use_stratagem(name, pos, on_done)` calls a stratagem from the slots of the bot the way a player does: the stratagem button held, the code entered, the flare thrown at `pos`. It spends the cooldown and a use, and `on_done(ok, reason)` hears how it ended. `stratagems()` lists what the bot carries, and `cancel_stratagem()` stops a call.

To drive one yourself: `move(dir)`, `aim(dir)`, `fire(on)`, `sprint(on)`, `reload()`, `dodge()`, `melee()`, `interact()`, `throw()` and `switch()`. Directions are world directions as `{ x, y }`. `unit()`, `alive()` and `remove()` do what they say. Bots walk straight at a goal with no pathfinding, and the game keeps local players on screen, so a player bot stops at the edge of the view.

Bots go before the results of a mission, so they never reach the end screens or the save of anyone.

## Rules worth knowing {#rules}

> [!WARNING]
> The game keeps hashed copies of guarded values such as `hitpoints_real`, `rounds`, `cooldown_timer`, `uses_left`, `num_grenades` and `extra_lives`. Writing one directly trips the anti-tamper check of the game, which cuts the player off from the network until the game restarts. Use `api.game.protected` or a queued command. `api.edits` refuses those keys.

- `api.game.command(unit, component, command, data)` queues a command on a unit this player owns, from a list of known ones. Queuing an unknown command straight into the game stops it with an error no script can catch.
- A timer that the game runs while above zero, such as the `drop_time` of a hellpod, never finishes at 0. Use 0.01.
- Spawning a unit whose package isn't loaded stops the game. `api.enemies.available(enemy)` checks.
- `api.console.run` refuses commands that reach the shared backend, wipe saves or unlock paid content. `spawn_enemy` runs only when nobody else is in the lobby, because it loads a package on the game of this player alone.
- Think twice before changing XP: the total goes up with the next ordinary results, even after a purge, and reaches the shared leaderboards.
