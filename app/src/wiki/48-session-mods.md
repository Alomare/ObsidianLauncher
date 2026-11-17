---
title: Writing session mods
group: Making mods
summary: How to change the shared game so the guardrails keep working for you.
---

A session mod changes something other players notice: damage, enemies, spawns, missions. The loader decides when such a mod may run ([Playing online](wiki:multiplayer)), and switches it on and off as players come and go. The job of the mod is to make every change through the loader, so the loader can switch it off.

A mod whose changes all happen inside content of a separate kind, such as a game mode with a separate mission screen, can be an [expansion](#expansions) instead, and leave every other mission alone.

## Set the scope

```json
"scope": "session"
```

Leaving the field out makes the mod unspecified, which runs as a client mod. Only use `client` when nothing the mod does reaches other players.

## Change the game through hooks

Hooks of a session mod call the original game code by themselves while the lobby isn't cleared. A mod built only from hooks needs no extra code to be safe:

```lua
-- Double the damage of every shot, in cleared lobbies only.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)
```

The same goes for `on_frame` functions and keys: in a lobby that isn't cleared, they don't run.

## Undo what stays behind

Some changes outlive the call that made them: a value written into a settings table, a unit you spawned. Listen for the switch and put things back:

```lua
local original_speed

obsidian.on_require("lua/settings/player_settings", function(settings)
  original_speed = settings.run_speed
end)

obsidian.on_session(function(on)
  local settings = rawget(_G, "PlayerSettings")
  if settings and original_speed then
    settings.run_speed = on and original_speed * 1.5 or original_speed
  end
end)
```

## Hybrid Scopes {#hybrid-scopes}

A session mod often has code only this player sees, such as a camera, next to code that changes the shared game. Keep `session` as the primary scope, declare [`secondary_scopes`](wiki:manifest#hybrid-scopes) and tag the code of each, and the personal code keeps running when the lobby stops being cleared:

```json
"scope": "session",
"secondary_scopes": [
  { "id": "view", "scope": "client", "name": { "en": "Camera" } },
  { "id": "weapon_range", "scope": "session", "option": "max_weapon_range", "name": { "en": "Max weapon range" } }
]
```

```lua
local VIEW = { secondary_scope = "view" }
obsidian.hook("CameraManager", "update", update_camera, VIEW)
obsidian.on_frame(steer, VIEW)
obsidian.hook("WeaponSettings", "range", longer_range, { secondary_scope = "weapon_range" })
obsidian.on_session(function(on) restore_range(on) end, { secondary_scope = "weapon_range" })
```

- A client secondary scope follows the rules of a client mod: only what this player sees, hears or presses. A camera is personal; time scale and gravity are not, even though they run on one machine.
- When the session secondary scopes switch off, the game tells the player which ones, and that the rest keeps working.
- A secondary scope with `option` is on only while that toggle is on. With every session secondary scope off, the mod deploys as a client mod, so the player gets Galactic War results back.
- A library the mod requires runs as a session mod while a session secondary scope is on, and treats the whole mod as a session mod in the calls it guards.
- One Lua state holds every secondary scope. Code in a client one can still reach session code directly, so read what session code changed each time instead of keeping a copy.
- An expansion can declare a session secondary scope too, for code that also runs outside the content in a cleared lobby. Untagged code of an expansion stays with the content.
- [Loader API: Hybrid Scopes](wiki:loader-api#hybrid-scopes) lists the arguments.

## Things to avoid

- **Replacing globals directly.** `SomeTable.func = mine` keeps running in every lobby, because the loader can't switch it off. The loader log lists such changes for every mod at startup.
- **Replacing game scripts with resources for gameplay changes.** A replaced script can't be switched off live either. Obsidian accepts it, but in a public lobby with a player who blocks session mods, the change stays in the game while the other session mods switch off.
- **Sending custom network messages** to other players outside the game code.

> [!NOTE]
> Changing the progress of the player who runs the mod, such as XP or unlocks, is allowed. Obsidian backs up the save before every deploy and doesn't undo those changes.

## Writing a library {#libraries}

A mod that exists for other mods to build on can declare itself a library:

```json
"scope": "library"
```

On each deploy, Obsidian makes it a session mod if an enabled session mod requires it, directly or through another library. Otherwise it is a client mod. Anything it does as a session mod goes through the usual rules, and it joins the set players compare. A library that replaces game files with `resources` keeps them in both cases, so keep gameplay changes out of those.

The library code has to work both ways. `obsidian.scope()` says which one this deploy gave, and `obsidian.session.guard` sorts the functions the library hands to other mods. Name the ones that are safe for anyone: reading, undoing, and changes only this player sees or hears. Every other function answers `false, "needs_session"` when the library runs as a client mod, and `false, "not_cleared"` in a lobby that isn't cleared. A function left off the list is refused.

```lua
local M = obsidian.module("lib/enemies")

-- The gameplay hooks wait for a session mod that needs them.
if obsidian.scope() == "session" then
  obsidian.hook("Damage", "apply", M.on_damage)
end

return {
  enemies = obsidian.session.guard(M, { "list", "count" }),
}
```

The library itself keeps the real table, so it can still undo what it changed after the lobby stops being cleared.

## Mods every player needs {#every-player}

The `consent` of a player tells the mod that the player allows it, not that the player runs it. That is enough for most mods, since the game of the host decides what happens. A mod that only works when every game in the lobby has it, such as one that changes how many players fit in a lobby, can ask for more:

```json
"scope": "session",
"every_player": true
```

Such a mod runs only when every other player runs it too, at the same version and with the same shared option values. Players who only allow it don't count. Obsidian switches it off, and back on, like any session mod, and tells the player why.

`obsidian.session.cleared()` answers for the mod that asks, and `obsidian.on_session` calls the mod when the switch of that mod changes.

The check runs once a player is in the lobby, and anyone can join first, so leave the game as it was until every player is confirmed: for a bigger lobby, keep the usual limit until `obsidian.session.cleared()` is true, and go back to it when `on_session` says `false`.

## Writing an expansion {#expansions}

An expansion is content that stands apart from the normal game, such as an arcade mode. Outside the content it counts as a client mod: players keep normal matchmaking, the Galactic War and Proving Grounds. Only the missions the content runs in leave the shared war ([Scopes](wiki:scopes#expansions)).

```json
"scope": "expansion",
"min_obsidian": "1.2.0",
"requires": ["obsidian.api"]
```

`every_player` doesn't go with it: the content only runs when every player runs the expansion anyway.

### What runs when

| | Outside the content | While the content runs |
|---|---|---|
| `obsidian.hook`, `obsidian.on_frame` and keys | Off: hooks call only the original function | On, in every game in the lobby |
| `obsidian.wrap`, and keys with `opts.always` | On | On |
| `obsidian.ui`, menu pages and overlays | On | On |
| Guarded functions of a library, such as most of Obsidian API | `false, "not_started"` | Full use |
| `obsidian.session.cleared()` | `false` | `true` |

The way into the content, a menu page or a screen drawn with `obsidian.ui`, works in any lobby. The content itself goes in hooks.

### Starting and stopping

```lua
-- An Obsidian from before expansions would run every hook in every lobby.
if not obsidian.capabilities.expansion then return end

local api = obsidian.import("obsidian.api")

api.menu.add("Arcade", function(page)
  page:action("Start", function()
    local ok, why, names = obsidian.expansion.start()
    if ok then return "Started" end
    if why == "players_missing" then return "Missing: " .. table.concat(names, ", ") end
    return why
  end)
  page:action("Stop", function() obsidian.expansion.stop() end)
end)

-- Only while the content runs, in every game in the lobby.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)

obsidian.on_session(function(on)
  if on then set_up() else undo() end
end)
```

`obsidian.expansion.start()` works on the game of the host. It checks that every other player in the lobby runs the same expansion, at the same version and with the same shared option values. Then the content starts, and every other game in the lobby follows within about a second. `obsidian.on_session(true)` runs in each of them, and `obsidian.on_session(false)` once the content ends, so those are the places to set up and undo. The other games learn about the content only that way, from the host.

The answer is `true`, or `false` and a reason:

| Reason | When |
|---|---|
| `"players_missing"` | Someone in the lobby doesn't run it. A third value lists their names, and the host sees them in a notice |
| `"not_host"` | Another player hosts the lobby |
| `"busy"` | The content of another expansion runs. One runs at a time |
| `"not_expansion"` | The calling mod isn't an expansion |

`obsidian.expansion.stop()` ends the content, on the game of the host, and answers `false, "not_running"` when it isn't running. Leaving the lobby ends it too. When the host leaves, the player who takes over the lobby carries the content on. `obsidian.expansion.running()` says whether the content of the calling mod runs now, and `running(id)` asks about another mod.

### Players who join

While the content runs, the lobby of the host shows in matchmaking only to players who run the same expansion at the same version. A player who tries to join without it, through an invite or a friend, is turned away before loading in, and the host sees the name. Someone who reaches the mission without it anyway is removed with the kick built into the game, and can come back once the content ends. Players joining or leaving never turn the content off for the players already in it.

### Libraries an expansion requires

A library that an expansion requires counts as a client mod at deploy, unless a session mod requires it too. `obsidian.scope()` answers `"session"` in it, so the library sets up everything an expansion may use, and the rest of the time it runs as a client library would. While the content runs, the guarded functions of the library work for the expansion, and `obsidian.on_session` tells the library when the content starts and ends, so it can undo what changed. Client mods using the same library still get `"needs_session"` from those functions.

### Things to know

- Files replaced with `resources` are in the game all the time, content or not. Keep them to what a client mod could change, such as fonts and images.
- Shared option values count, as in a session mod. Mark the personal ones `"scope": "client"`.
- The results of a mission the content ran in never reach the Galactic War, even when the content stops before the results screens. The next mission without the content counts again.
- `min_obsidian` keeps Obsidian 1.1 from deploying the mod, and asks the player to update. Obsidian 1.0 doesn't read it and runs the mod as an unspecified one, so the capability check at the top of the entry script keeps the mod out of the game there.

## Options in session mods

Every option value counts when Obsidian compares players' sets, unless you mark it `"scope": "client"` or put it in a client secondary scope. The same goes for an expansion. Mark the options that only change what this player sees ([Options and keys](wiki:options-and-keys#personal-options-in-session-mods)).
