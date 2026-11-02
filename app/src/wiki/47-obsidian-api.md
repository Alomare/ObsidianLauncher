---
title: Obsidian API
group: Making mods
summary: A library for mods: the game's data by name, checked changes, and ready actions.
---

Obsidian API is a mod other mods build on. It knows where things are in the game, makes changes in ways the game accepts, and undoes them when it has to. It's how Obsidian Trainer works, and the trainer uses nothing you can't use.

## Using it

List it in `requires`, and Obsidian switches it on with your mod and starts it first:

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

Obsidian API is a [library](wiki:session-mods#libraries). A client mod can require it and stay a client mod, with no effect on matchmaking or the Galactic War. In that case it can read the game's data and use `api.ui`, `api.audio` and `api.events`, and anything that changes the shared game answers `false, "needs_session"`. To use those, make your mod a session mod.

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
-- Double the assault rifle's damage, upgrades included.
local ok, why = api.edits.set_unit("equipment/wep_assaultrifle_01", "weapon_info.power", 150)

-- A value in any table the game reads.
local s = api.catalog.stratagem("stratagem_resupply")
api.edits.set_table("Resupply", s, "cooldown_time", 30)
```

- Types and ranges are checked, and `opts` can give `min` and `max`.
- Tables stay the same tables, because live units hold references to them.
- A weapon has a copy of its settings for each upgrade, and an edit reaches them all.
- `api.edits.list()`, `revert(id)` and `revert_all()` show and undo changes.
- `opts.also` in `set_table` lists more paths that take the same value, as one change.
- When the lobby stops being cleared, every edit is undone. When it clears again, they return.

A stratagem's upgrade levels can set a field for themselves, so a change to the base stops at those levels. `api.stratagems.edit` changes every level at once, with the ranges of the field:

```lua
api.stratagems.edit("stratagem_railcannon_strike", "cooldown_time", 30)
```

Fields a player can edit, with ranges and when each takes effect, are in `api.catalog.fields(family)`. `api.catalog.families_of(settings)` says which families a settings table has. Some numbers apply at once and some at the next spawn. `api.gear.refresh()` pushes the cached ones into weapons in play.

## Effects {#effects}

An effect stays on across respawns and missions. The game rebuilds much of its state then, so the library applies active effects again.

```lua
api.effects.set("god", true)
api.effects.set("speed", 2)     -- a value, false to switch off
api.effects.is_on("god")
```

Your own:

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

`api.events.on(name, fn)` calls your function when the game says something happened, so you don't have to look every frame.

```lua
local kills = 0

api.events.on("on_enemy_killed", function(unit, attacker, owner)
  kills = kills + 1
  if kills == 50 then api.ui.hint("Fifty down") end
end)
```

| Event | Your function gets | When |
|---|---|---|
| `on_enemy_killed` | `unit`, `attacker`, `owner`, `damage_type` | This player got a kill. `damage_type` is there only when this game worked out the hit |
| `on_enemy_spawned` | `unit`, `enemy_name` | An enemy appeared, on every player's game |
| `on_player_death` | `unit` | A player died, on every player's game |
| `on_objective_completed` | nothing | An objective was finished |
| `on_stratagem_activated` | `peer`, `stratagem`, `position`, `unit` | A stratagem's countdown ended, on every player's game |
| `on_avatar_extract` | `unit`, `shuttle` | This player boarded the shuttle |
| `on_killed` | `unit`, `attacker`, `owner` | A unit with health was killed |
| `on_dead` | `unit` | A unit's death finished |
| `on_health_downed` | `unit`, `state` | A player went down |
| `on_damage_taken` | `unit`, `hit` | A unit lost health. `hit` has `damage`, `hit_zone`, `attacker_unit` and `owner_unit` |
| `on_health_received` | `unit`, `amount` | A unit was healed. A full heal passes neither |
| `on_stratagem_pre_activated` | `stratagem`, `position`, `owner`, `flare` | A flare's countdown started |
| `on_stratagem_used` | `unit`, `data` | A player used a stratagem |
| `on_stratagem_wielded` | `unit`, `flare`, `info` | A player took a flare in hand |
| `on_extract_shuttle_landed` | `unit` | The shuttle landed |
| `on_add_to_stat` | `unit`, `stat`, `amount` | A statistic went up, such as `"stratagems_used"` or `"hits"`. This one comes often |
| `on_pause` | `paused` | The pause menu opened or closed |
| `on_mission_end` | `stats`, `extra` | The end screens have the results |

- A function that fails counts against your mod, and the others still run.
- Nothing arrives while the lobby isn't cleared.
- `position` and `hit` last one frame. Copy what you need from them.
- `api.events.off(name, fn)` stops one.

## What's in the API {#contents}

| Table | For |
|---|---|
| `api.game` | The current state, the local player, `is_host()`, and safe writes |
| `api.catalog` | Weapons, stratagems, perks, enemies and groups by name: the base game, plus the DLC the player owns. `usable(name)` checks that, `unowned_path(path)` checks a weapon's path, and `unlocked(name)` says whether the player has unlocked an item by playing. Also `backpacks`, `crates`, `pickups`, `vehicles`, `by_subtype(name)` and `max_upgrade(path)` |
| `api.player` | `heal`, `heal_by(amount)`, `set_health(n)`, `respawn`, `teleport`, `set_cosmetic`, `set_outline(r, g, b)`, `clear_status(name)`, `immune(name, on)`, `knockdown`, `drop_carried`, and more |
| `api.gear` | `refill`, `give_weapon(slot, path, upgrade)`, `give_backpack(path, upgrade)`, `give_perk`, `remove_perk`, `refresh`, `weapons_of(owner)`, `tune(unit, component, changes)`, `add_drone(path, unit, upgrade)` |
| `api.stratagems` | `reset_cooldowns`, `refill_uses`, `give`, `drop`, `call(name, pos, delay)`, `edit(name, field, value)` |
| `api.enemies` | `spawn`, `spawn_group`, `spawn_entity(path, pos)`, `kill_all`, `kill(unit)`, `list`, `info(unit)`, `nearest(pos)`, `apply_status(unit, template)`, `stun_near(radius)`, `near(radius, pos)`, `damage(unit, power, statuses)`, `set_hitpoints(unit, n)`, `despawn(unit)`, `load_faction(name)`, `set_pressure(t)`, `waves()`, `force_wave(name, pos)`, `carried()`, the enemies on the map with corpses and the spawn queue, and `room()`, how many more fit under `MAX_SAFE` |
| `api.mission` | `finish_objectives`, `objectives()`, `mutator(name)`, `win`, `abort`, `practice` |
| `api.vehicles` | `current()`, `driving()`, `repair()`, `refill()` |
| `api.combat` | `scale{ players, enemies }` |
| `api.events` | `on(name, fn)`, `off(name, fn)` |
| `api.ui` | `hint(text, seconds)`, `ping(pos)`, `open(name, opts)`, `close()`, `menus()`, `with_texts(texts, fn, ...)` |
| `api.lobby` | `players()`, `share(name, value)`, `shared(name)` |
| `api.world` | `cursor`, `ahead`, `to_screen(pos)`, `prop(unit, pos, opts)`, `explode`, `camera`, `shading(name, kind, value)`, `flash(seconds)` |
| `api.audio` | `bus(name, db)`, `play(event, pos)`, `stinger(name)` |
| `api.progress` | `set_samples`, `unlock_items`, `upgrade_all`, `loadout()`, `set_loadout(t)`, `upgrades()`, `set_upgrade(name, value)`, `upgrade_limit(name)` |
| `api.console` | `run(line)`, `commands()` |

## Player {#player}

These effects stay on across respawns. Most give back what was there before, so a perk's own value returns when they come off.

| Effect | Value |
|---|---|
| `armor` | Armor at every angle |
| `sprint_speed` | Top sprint speed |
| `throw_range` | A multiplier on throws |
| `acceleration`, `deceleration` | How fast the player starts and stops |
| `ignore_slow` | `true`: the mover's slow multiplier doesn't apply, so nothing that works through it slows the player |
| `no_screen_leash` | `true`: the camera stops holding the player inside the screen |
| `hide_model` | `true`: the player's model is hidden |
| `heavy_armor_look` | `true`: the perk's look, without its armor |

- `api.player.set_outline(r, g, b)` changes the outline the player shows through walls, and `set_outline(nil)` gives it back.
- `api.player.clear_status(name)` takes off a status template, or every status of an element such as `"fire"`. `api.player.immune(name, on)` keeps one off across respawns, until `on` is `false`.
- `set_cosmetic("gender", index)` changes the character, with an index from `api.catalog.cosmetics("gender")`.

## Gear {#gear}

`upgrade` in `give_weapon` and `give_backpack` runs from 0 to 15. Leave it out for what the player owns for that item. `api.catalog.max_upgrade(path)` gives the highest one.

`give_backpack` takes only paths from `api.catalog.backpacks()`, because the game reads settings other paths don't have, where nothing can catch the error. The old pack drops to the ground, as the old weapon does.

`give_perk` changes perks the way the game's console does. Laser sight answers `"alone_only"` with other players in the lobby, because their games would run it with no setup.

### One weapon's stats

`api.data` edits change a weapon's type for everyone on that machine. `api.gear.weapons_of(owner)` lists the weapons a player holds, as `{ unit, component, slot, info }`, and `api.gear.tune(unit, component, changes)` changes one of them alone. `changes` holds a factor for a number (`firerate = 1.2`) and `true` or `false` for an attribute (`armorpiercing = true`). Each call replaces the last, and `{}` takes it all off. It reaches only the weapons this machine owns, so each player's game tunes their own.

The component `"melee"` tunes the same weapon's melee attack instead: `power`, `duration` (how long the swing holds the player, so a smaller one swings faster) and `delay` (how soon it hits).

```lua
for _, w in ipairs(api.gear.weapons_of()) do
  api.gear.tune(w.unit, w.component, { firerate = 1.2 })
end
```

### Drones without a backpack

`api.gear.add_drone(path, unit, upgrade)` gives a player a drone pack's drones and leaves their backpack slot alone. `path` is a drone pack from `api.catalog.backpacks()`, such as `equipment/equipment_drone_defensive_backpack_01` or `equipment/equipment_drone_heal_backpack_01`. The drones need the pack's station to dock, reload and fly around, so the station rides on the player as a worn pack does, and every game running Obsidian API hides it. Call it again for more drones. It answers a drone with `:alive()` and `:remove()`. The drones dock while their player sits in a vehicle, and go when the player dies, as a worn pack drops, so give them again after a respawn.

```lua
local dog = api.gear.add_drone("equipment/equipment_drone_defensive_backpack_01")
```

## Stratagems {#stratagems}

`api.stratagems.call` puts down a real flare, as the game does for its own timed drops. It has the sound and the countdown, patrols hear it, and it works for Reinforce. It spends no cooldown or use, and it's called at the level the player owns. A fourth argument, a unit, calls it for another player on this machine: the call-in and its kills are theirs.

```lua
api.stratagems.call("stratagem_resupply", api.world.cursor(), 15)   -- 15 s on top of its own call-in
```

`api.stratagems.drop` is the quick one: no flare and no wait, and no Reinforce.

The effect `ignore_jammers` lets stratagems work inside no-stratagem zones.

## Enemies {#enemies}

```lua
local bug, metres = api.enemies.nearest()
if bug then api.enemies.apply_status(bug, "flashbang_stun") end

-- Bigger fights on the same mission. false gives a number back to the game.
api.enemies.set_pressure({ cap = 24, difficulty = 15, patrols = 20, interval = 4 })
api.enemies.set_pressure({ difficulty = false })
```

- `apply_status` works on any enemy from any player's game. Status names are in `api.catalog.status_templates()`.
- `info(unit)` gives `key`, `name`, `faction`, `boss`, `hitpoints`, `max_hitpoints` and `owned`.
- `set_pressure` is the host's. `cap` runs up to 32 and lifts both limits waves stop at. Corpses count.
- `difficulty` is what each encounter can spend. The seed, the waves on offer and the objectives stay the same.
- `force_wave` spawns a wave from `api.enemies.waves()` all at once, ignoring its delays.
- `spawn(key, count, where)` takes `where` as `"cursor"`, a position (a vector or `{ x, y, z }`), or nothing for in front of the player.
- `spawn_entity` spawns a crate or pickup. Take paths from `api.catalog.crates()` and `pickups()`, which leave out paid items.
- `near(radius, pos)` lists living enemies, nearest first, around a position or the player.
- `damage(unit, power, statuses)` is a hit from this player that works from any player's game, and armor counts. `statuses` is an optional list of status names.
- `set_hitpoints(unit, n)` and `despawn(unit)` go through whichever player's game owns the enemy.
- `load_faction(name)` loads `"bugs"`, `"cyborgs"` or `"illuminates"` during a mission, so any of them can spawn on any planet. It answers `"alone_only"` with anyone else in the lobby, because every game needs the package. The game stops for a moment while it loads, and it's unloaded when the mission ends.

## Missions {#missions}

`api.mission.objectives()` lists `name`, `text`, `type`, `state`, `group`, `done`, `total` and `position`. `state` is `pending`, `complete` or `failed`.

The effect `protect_objectives` keeps convoys and escorted people alive. It reaches what this player's game runs, so use it on the host.

`api.mission.mutator(name)` turns on one of the game's proving ground modifiers in any mission, and `nil` turns it off. There's one at a time, and a real proving ground keeps its own.

| Name | What it does |
|---|---|
| `super_boots` | 40% faster on foot |
| `what_goes_around` | Damage to a teammate comes back to the shooter |
| `orbital_support` | Support stratagems land near a player every 20 s |
| `orbital_bombardments` | Strikes land near a player every 15 s, plus their call-in |

## Vehicles {#vehicles}

These effects reach the vehicle the player is driving, and let go of it when they get out. A passenger's game can't change it.

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

A multiplier from 0 to 100, after armor, so armor that stops a shot still stops it. 1 or `false` turns a side off. The game passes no attacker to its sums, so a scale goes by who takes the hit. Each player's game works out its own hits, so everyone in a lobby should use the same numbers.

## Camera and look {#view}

All of this is this player's alone: nobody else sees it. Obsidian API is a session mod, so it stops while the lobby isn't cleared.

```lua
api.world.camera.fov(90)                    -- degrees, nil gives it back
api.world.camera.spin(30)                   -- degrees around the player
api.world.camera.spectate(friend_unit)      -- nil comes back
api.world.camera.photo(true)                -- freezes the camera and clears the screen
api.world.camera.place(position, rotation)  -- moves it, in photo mode

api.world.shading("fog_color", "vector3", { 0.5, 0.45, 0.3 })
api.world.shading("fog_color", "vector3")   -- back to the planet's
api.world.flash()                           -- lightning
```

- A low spin or angle can make aiming feel odd: the mouse aims in screen space.
- `spectate` takes another player's unit, from `api.game.all_avatars()`.
- `shading` takes the names in `api.world.SHADING`, by kind: `scalar` is a number, `vector2` and `vector3` are lists. A value goes on again in each new mission. The game only ships code for `fog_color`, `sun_color` and the other `vector3` names, with no use of its own, so check them on your first run.
- `flash` needs a forest planet, and not one of the illuminates. Others answer `"wrong_environment"`.

## Sharing between machines {#lobby}

`api.lobby.share(name, value)` puts a text of up to 200 characters where every other machine in the lobby can read it, under your mod's id. `api.lobby.shared(name)` answers what each other machine shares, by peer. Steam passes a change on within about a second, so it suits state that changes now and then, such as a score, not every frame. Without a lobby, `share` answers `false, "not_available"`.

```lua
api.lobby.share("score", tostring(my_score))
for peer, text in pairs(api.lobby.shared("score")) do total = total + tonumber(text) end
```

`api.world.to_screen(pos)` answers where a point of the level is on the screen, in pixels from the bottom left, and whether it's in view, for drawing over the world with a kit.

## Props {#props}

`api.world.prop(unit, pos, opts)` puts a unit of the game in the level on this machine only. Nothing goes over the network and no game system knows about it, so it does nothing but be seen, with whatever glow and highlight the unit has. `opts.scale` sets its size, 1 as made. It answers a prop with `move(pos)`, `scale(size)`, `flow(event)`, `alive()` and `remove()`, or `nil` and a reason. `flow` runs one of the unit's own flow events, the way the game makes a unit play its sounds and effects: `on_picked_up` on a research sample plays the pickup.

```lua
local sample = api.world.prop("gameobjects_spec/gameobject_bug_research_object", pos, { scale = 2 })
sample:move(new_pos)
sample:remove()
```

The unit has to be in a package the level has loaded: the game stops for one it can't find. A mission's research samples, `gameobjects/flare_01` and the enemies of a loaded faction are safe.

## Sound {#sound}

Sound is local too.

```lua
api.audio.bus("combat_enemy", -30)   -- decibels; 0 is as mixed, -60 is silent
api.audio.bus("combat_enemy", false) -- back to the game's mix

api.audio.play("ui_select")          -- anywhere
api.audio.play("ui_select", position)
api.audio.stinger("music_objective_found")
```

- `api.audio.BUSES` lists the buses. A volume repeats every frame, because the game's mix states write them as they blend.
- `play` and `stinger` answer `"unknown_event"` for a sound this mission hasn't loaded.
- `stinger` goes through the game's own cooldown of 5 seconds.

## Loadouts {#loadouts}

```lua
api.progress.set_loadout({ primary_weapon = "weapon_railgun", stratagems = { [1] = "stratagem_resupply" } })
api.progress.set_upgrade("weapon_railgun", 3)
```

Both write the real save, on the ship, with its menus closed, and answer `"close_screen"` otherwise.

- `set_loadout` takes any of `perk`, `primary_weapon`, `sidearm_weapon` and `stratagems`, which maps the four free slots to names. It's applied at the next spawn, and it's put in the first preset as well, because the Loadout screen starts from that.
- Paid items and unknown names go back to the default, as when the game reads a save. An item the player hasn't unlocked doesn't survive the Loadout screen.
- A value in `set_upgrade` is what the game keeps: a bit per upgrade for weapons, a level for stratagems. `upgrade_limit(name)` gives the highest one.

## Messages {#messages}

`api.ui.hint(text, seconds)` shows the game's own hint box, even with hints turned off. `text` is your words, or a hint name such as `"hint_alarm"`. Your words go through [`obsidian.notify`](wiki:loader-api#obsidian-notify-text-seconds), so it can answer `false, "queue_full"`. `api.ui.ping(pos)` puts this player's mark on the map at a position, and `api.ui.ping(nil)` takes it away. Every player sees it.

`api.ui.open(name, opts)` opens a menu of the ship by name, as a terminal does, and `api.ui.close()` closes the top one. `api.ui.menus()` lists the names. `opts.seed` gives Loadout and the briefing a mission, and `opts.pad` opens it for a pad other than the main player's.

`api.ui.with_texts(texts, fn, ...)` runs `fn` with some of the game's text keys showing your words: `texts` is `{ key = words }`. It only lasts while `fn` runs, so you can wrap one of the game's screens and reuse it as your own, while every other text stays the game's.

```lua
api.ui.with_texts({ menu_campaign = "SURVIVORS" }, original, self, ...)
```

`api.lobby.players()` lists `peer`, `name`, `ping` in milliseconds, `host` and `me`. It only reads. A teammate has a name during a mission.

## Rules worth knowing {#rules}

> [!WARNING]
> The game keeps hashed copies of guarded values such as `hitpoints_real`, `rounds`, `cooldown_timer`, `uses_left`, `num_grenades` and `extra_lives`. Writing one directly trips its anti-tamper check, which cuts the player off from the network until they restart. Use `api.game.protected` or a queued command. `api.edits` refuses those keys.

- `api.game.command(unit, component, command, data)` queues a command on a unit this player owns, from a list of known ones. Queuing an unknown command straight into the game stops it with an error no script can catch.
- A timer that the game runs while above zero, such as a hellpod's `drop_time`, never finishes at 0. Use 0.01.
- Spawning a unit whose package isn't loaded stops the game. `api.enemies.available(enemy)` checks.
- `api.console.run` refuses commands that reach the shared backend, wipe saves or unlock paid content. `spawn_enemy` runs only when nobody else is in the lobby, because it loads a package on this player's game alone.
- Think twice before changing XP: the total goes up with the next ordinary results, even after a purge, and reaches the shared leaderboards.
