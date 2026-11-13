---
title: Loader API
group: Making mods
summary: Every function of the obsidian table, version 1.
---

Obsidian Loader starts the `entry` script of each enabled mod in load order, once, when the game boots. Every script sees a global table named `obsidian`. Everything a mod registers through it runs protected: an error is logged and never stops the game.

## Hooks {#hooks}

### obsidian.hook(target, method, fn, opts)

Wraps a function of the game. `target` is a table, or the name of a global table as a string. `fn` receives the original function first, then the arguments of the call:

```lua
obsidian.hook("PlayerUnitMovement", "update", function(original, self, dt)
  -- before the game code
  local result = original(self, dt)
  -- after the game code
  return result
end)
```

When the global doesn't exist yet, the hook waits and goes in as soon as the game defines it. If `fn` fails, the original runs instead.

In a session mod, the hook calls only the original while the lobby isn't cleared ([Writing session mods](wiki:session-mods)). In an expansion, it calls only the original until the content starts ([Writing an expansion](wiki:session-mods#expansions)). `opts.secondary_scope` puts the hook in a [secondary scope](#hybrid-scopes) of the mod.

### obsidian.wrap(target, method, fn)

The same as `obsidian.hook`, with the same safety if `fn` fails, except that it keeps running while the lobby isn't cleared, and outside the content of an expansion. It's for the way into an expansion, and for a mod whose replaced files can't be switched off either, where a hook going off would leave one game half modded. Anything else should use `obsidian.hook`. Check `obsidian.capabilities.wrap` on an older loader.

### obsidian.on_require(name, fn)

Calls `fn(module)` when the game loads the script `name`, such as `"lua/settings/perk_settings"`. Use it to change a settings table right after the game builds it. When the script has already loaded, `fn` runs at once.

### obsidian.on_frame(fn, opts)

Calls `fn` once per frame. Return `true` to stop the calls. The frame functions of a session mod only run in a cleared lobby, and those of an expansion only while the content runs. `opts.secondary_scope` puts the function in a [secondary scope](#hybrid-scopes) of the mod. `opts.always` runs it in any lobby, as `obsidian.wrap` does a hook, for work only this player sees; check `obsidian.capabilities.secondary_scopes` first, since an older loader gates it as before.

### obsidian.after_startup(fn)

Calls `fn` once every mod has started, so mods can work together without depending on load order.

## Settings and keys

### obsidian.settings.get(id, mod_id)

The value of option `id` of this mod: the one the player chose in the app, or the default ([Options and keys](wiki:options-and-keys)). A game script the mod replaces runs outside the mod, so give the id of the mod as `mod_id` there.

### obsidian.keybind(id, fn, opts)

Calls `fn` each time the key bound to `id` goes down. The key comes from the app, or from the default in `obsidian.json`. The keys of a session mod wait for a cleared lobby, and those of an expansion for the content, as the hooks of the mod do. `opts.always` leaves the key alone, as `obsidian.wrap` does a hook, for a key that only acts on the screen of this player. `opts.secondary_scope` puts the key in a [secondary scope](#hybrid-scopes) of the mod.

## Multiplayer

### obsidian.session.cleared(id)

`true` while the calling mod may run: the session is cleared, and for a mod marked `every_player`, every other player runs it too ([Mods every player needs](wiki:session-mods#every-player)). For an expansion, `true` while the content runs. Client mods rarely need it. With the id of a [secondary scope](#hybrid-scopes), the answer for that one: always `true` for a client secondary scope, and `false` for one that is off.

### obsidian.session.guard(t, free)

A view of table `t` to hand to other mods. The functions named in the list `free` work as usual. Every other function answers `false, "needs_session"` when the calling mod runs as a client mod, and `false, "not_cleared"` when the session isn't cleared ([Writing a library](wiki:session-mods#libraries)). An expansion calling them gets `false, "not_started"` outside the content, and full use of the libraries it requires while the content runs, whoever else is in the lobby.

### obsidian.session.members()

A list of the other players in the lobby. Each entry has `peer`, `obsidian` (whether that player runs Obsidian), `modset` and `consent` (true unless that player blocks other players' session mods in a public lobby).

To run only when every player has the mod, set [`every_player`](wiki:session-mods#every-player) in the manifest instead of comparing these yourself.

### obsidian.session.forfeit_progress(), release_progress(key) and hold_progress()

These do nothing. The save belongs to the player, so Obsidian holds back none of the progress from a mission, whatever the mods do. They stay so mods that call them keep working. Only Galactic War and leaderboard reports stay blocked, while session content is deployed and for missions the content of an expansion ran in ([The Galactic War](wiki:galactic-war)). To play without a mission counting, a player switches on **Freeze account progression** in [Guardrails](wiki:guardrails).

### obsidian.guardrails()

What the player chose in **Guardrails** for this launch, as `{ consent, block_achievements, freeze_progress }`, where `consent` is false while the player blocks other players' session mods in a public lobby. A mod can read them, for example to tell the player their account is frozen. Only the player can change them, and the loader enforces them under every mod ([Guardrails](wiki:guardrails)).

### obsidian.war()

The custom war from [Commander](wiki:commander) this launch plays, as `{ id, model }`, where `model` is `"vanilla"` or `"enhanced"`; `nil` on the official war. The companion mod of an Enhanced war reads it to know when to act. Check `obsidian.capabilities.war` first: an older loader has no such call.

### obsidian.on_session(fn, opts)

Calls `fn(on)` whenever the loader switches this mod on or off, so the mod can undo what other players would see. A mod marked `every_player` also hears when a player who doesn't run it comes or goes. An expansion hears `true` when the content starts and `false` when it ends, in every game in the lobby, and so does a library that only expansions require. With `opts.secondary_scope`, `fn` hears only the switch of that [secondary scope](#hybrid-scopes): a client one never switches off.

### Hybrid Scopes {#hybrid-scopes}

A mod with [`secondary_scopes`](wiki:manifest#hybrid-scopes) tags each registration with the secondary scope it belongs to, as the last argument of `obsidian.hook`, `obsidian.on_frame`, `obsidian.keybind` and `obsidian.on_session`:

```lua
local view = { secondary_scope = "view" }
obsidian.hook("CameraManager", "update", on_camera, view)
obsidian.on_frame(tick_view, view)
obsidian.keybind("toggle_view", toggle, { secondary_scope = "view", always = true })
obsidian.on_session(function(on) if not on then undo_range() end end, { secondary_scope = "weapon_range" })
```

The code of a client secondary scope runs in any lobby, the code of a session one only in a cleared one. In an expansion, the code of a session secondary scope runs while the content runs, and outside it in a cleared lobby. In a session mod, untagged code follows the strictest secondary scope that is on; in an expansion, it stays with the expansion. A secondary scope that is off registers nothing. Code that runs once, such as the entry script and `on_require`, isn't tagged. One Lua state serves every secondary scope, so code in a client one can still call session code directly: only the registrations are gated.

`obsidian.scope()` answers `"hybrid"` while a session mod has a client and a session secondary scope on. The calls a library guards treat a hybrid mod as a session mod, from any secondary scope. An older loader ignores `opts.secondary_scope` and gates everything with the primary scope, so check `obsidian.capabilities.secondary_scopes` before telling the player that the personal code keeps running. Loaders from before Hybrid Scopes had that name read `opts.part` instead, and this loader still does.

## Expansions {#expansions}

For a mod with `"scope": "expansion"` ([Writing an expansion](wiki:session-mods#expansions)). Check `obsidian.capabilities.expansion` first: an older loader doesn't have these.

### obsidian.expansion.start()

Starts the content of the calling expansion, on the game of the host. Every other player in the lobby has to run the same expansion, at the same version and with the same shared option values. Answers `true`, or `false` and a reason: `"players_missing"` with a list of the names of the players who don't run it as a third value, `"not_host"`, `"busy"` while the content of another expansion runs, or `"not_expansion"`. A refused start also shows the host a notice with the names.

### obsidian.expansion.stop()

Ends the content, on the game of the host. Answers `true`, or `false` and `"not_running"` or `"not_host"`. Leaving the lobby ends the content too.

### obsidian.expansion.running(id)

`true` while the content of the expansion `id` runs in this game. With no `id`, the calling mod.

## Text and logs

### obsidian.text(key, ...)

The text `key` from the `locales` folder of the mod, in the language of the game, with English as the fallback. Extra arguments fill `%s` and `%d` placeholders. A catalog is named for the language: `en.json`, `fr.json`, `es.json`, `pt-BR.json`, `de.json`, `it.json`, `ru.json`, `pl.json`, `ko.json`, `ja.json`, `zh-CN.json`, `zh-TW.json`, `sv.json`, `da.json`, `nl.json`, `fi.json` or `no.json`.

### obsidian.language()

The language the game runs in, by those same names, such as `"ru"`. `"en"` for a language Obsidian doesn't know. Check `obsidian.capabilities.language` on an older loader.

### obsidian.game_text(key, words)

Makes the game show `words` for one of the game text keys, or for a key the mod invents, such as the title of a new rank. `words` is a string, or a table by language such as `{ en = "Warden", ru = "Страж" }`, which falls back to English. `nil` gives the key back to the game. Answers `false` for anything else. A text is only what this player sees, so it keeps working while the lobby isn't cleared. Check `obsidian.capabilities.game_text` on an older loader.

### obsidian.log(...)

Writes a line to the log of this mod, in `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`.

### obsidian.profiler.start(), stop(), running() and read()

Measures how much of each frame the scripts of every mod take. `start()` turns measuring on for the calling mod until it calls `stop()`; measuring stays on while any mod asked for it, and costs a little of every call into a mod. `read()` answers the last result, made about twice a second, or `nil` before the first one: `{ frame_ms, frames, total_ms, clock, mods = { { id, ms, peak_ms, calls } } }`, the most costly mod first. `ms` is the average time a frame, `peak_ms` the most in one frame, and `calls` the average number of calls a frame. The time a hook spends in the game's own function is the game's, not the mod's. `obsidian.loader` is the loader's own hooks. The Script Profiler page of the F6 window shows these numbers over the game. Check `obsidian.capabilities.profiler` on an older loader.

## Other mods and more files {#other-mods}

### obsidian.module(name)

Loads another Lua file of this mod and returns what it returns. The file has to be listed in `modules` in `obsidian.json`, and `name` is the file path without `.lua`, such as `"lib/parts"`.

### obsidian.import(id)

What the `entry` script of another mod returned, or `nil` when that mod isn't running. List the mod in `requires`, so it starts first ([Obsidian API](wiki:obsidian-api)).

### obsidian.scope()

How the calling mod runs in this deploy: `"client"`, `"session"`, `"hybrid"` (a session mod with a client and a session secondary scope on, [Hybrid Scopes](#hybrid-scopes)) or `"expansion"`. A library gets `"client"` or `"session"`, and `"session"` when an expansion requires it, so the library sets up what the expansion may use ([Writing a library](wiki:session-mods#libraries)).

### obsidian.caller()

The id of the mod whose code is running, so a shared mod can record who asked for something.

### obsidian.info(id)

What a running mod is: `{ id, name, version, scope }`. `name` comes from the `obsidian.json` of the mod, in the game language, else English, and `scope` is what `obsidian.scope()` answers in that mod. With no `id`, the calling mod. `nil` for a mod that isn't running.

## In-game menu {#menu}

The in-game window belongs to Obsidian API, so it changes with the releases of the API, and new mods use `api.menu` ([Obsidian API](wiki:obsidian-api)). **F6** opens it, a key that belongs to Obsidian API and that players change in the Keys tab of Obsidian API, and the pages of each mod sit in a folder named after the mod. `obsidian.menu` stays for mods written before, and hands every call to the Obsidian API window, as the mod that made it. List `obsidian.api` in `requires`: with no Obsidian API running there's no window, and `open()` answers `false`.

### obsidian.menu.add(title, build, opts)

Adds a page to the list down the left of the in-game menu, in the folder of the mod. `title` is text, or a function that returns it. `build(page)` runs each time the page opens. `opts.icon` is one of `obsidian.ui.glyphs()`:

```lua
obsidian.menu.add("Example Mod", function(page)
  page:header("Tuning")
  page:toggle("Enabled", { get = function() return on end, set = function(v) on = v end })
  page:number("Power", { min = 1, max = 10, step = 1, get = get_power, set = set_power })
  page:choice("Mode", { choices = { { id = "a", label = "First" } }, get = get_mode, set = set_mode })
  page:action("Do it", function() return "Done" end)
  page:input("Name", function(text) end)
  page:page("More", function(sub) sub:label("Deeper") end)
end, { icon = "player" })
```

Any row takes `help`, a text shown while it's selected, and `enabled`, a function that returns `false` and a reason. A string returned by an action shows as a toast. An action can call `page:refresh()` to build the page again. A failing row shows the error and doesn't count against the mod.

A `toggle`, `number` or `choice` row takes `default`, the value it starts at. It then gets a reset button at the end of the row, lit once the value has moved, and R does the same from the keyboard. Where writing a value isn't the way back, give `reset`, a function that undoes it, and `changed`, one that says whether there's anything to undo.

The pages of a session mod aren't listed, and close, while the lobby isn't cleared. The pages of an expansion stay listed, so the content can be started from one.

### obsidian.menu.open(), close(), toggle(), is_open()

The Obsidian API key opens the window once a page is listed, so the mod needs no key for it. These are for a mod that opens the window at a moment it picks.

### obsidian.menu.overlay(fn)

Calls `fn(draw, width, height)` every frame, to put something on screen outside the menu. `draw.text(text, x, y, color)` and `draw.rect(x, y, w, h, color, alpha)` work in pixels from the bottom left. Colors are `{ r, g, b }`.

### obsidian.toast(text, seconds)

Shows a short message at the top left of the screen, or beside the in-game menu while it's open. With Obsidian API running, the API window draws it.

### obsidian.notify(text, seconds) {#obsidian-notify-text-seconds}

Shows `text` in the hint box of the game, with the hint icon and sound, even with game hints turned off. `seconds` is 7 if you leave it out. Where the game has no hint box, such as the main menu, it shows as a toast instead.

The box shows one message at a time, so use it for what's rare and matters, and `obsidian.toast` for quick feedback such as a key press. A text already waiting isn't added again, and a mod can have 3 waiting: past that, `notify` returns `false` and `"queue_full"`. Messages from Obsidian go first.

## Replacing game screens {#ui}

`obsidian.ui` is for a mod that redraws the game menus and HUD. Any mod can use it, of any scope, with nothing in `requires`, so a reskin stays a client mod and keeps the Galactic War and matchmaking. Nothing here runs until a mod asks for it.

The idea is to keep the game logic and input, stop the game drawing, read the game data and draw on top. The update and draw functions of the game always run, because keys, mouse and network code live inside them.

```lua
local ui = obsidian.ui
ui.frame(1920, 1080)

ui.on("loadout", {
  mute = true,
  draw = function(kit, data, dt)
    kit:panel{ 120, 140, 1680, 800, cut = 24, corners = "tr bl", fill = "#38383ee0", line = "#ffc433" }
    kit:text("LOADOUT", 960, 880, { font = "heading", size = 48, tracking = 4, align = "center" })

    for i, player in ipairs(data.players()) do
      local panel = data.loadout(player.slot)
      local x = 140 + (i - 1) * 420
      kit:band(x, 780, 400, 40, player.color)
      kit:text(player.name, x + 12, 792, { font = "body_large" })
      if panel then kit:text(panel.ready and "READY" or "PICKING", x + 12, 740) end
    end

    kit:text(ui.prompt("menu_loadout_select"), 140, 150)
  end,
})
```

If the code for a screen fails, that screen stops for the session, the game draws the original again, and the error goes in `loader.log`.

### obsidian.ui.current()

Where the player is: `{ state, screen, sub }`. `screen` is the id the rest of this section uses: a menu by name (`"loadout"`, `"armory_profile"`, `"main"`), `"title"` and `"ship"` with no menu open, `"hud"` in a mission, or the name of the state, such as `"mission_end"`. `sub` is the step inside it, when it has one.

### obsidian.ui.on(id, handlers), off(id)

Runs code while a screen is up. `id` is a value of `screen`, or `"*"` for all of them.

| Handler | When |
|---|---|
| `enter(screen)` | The screen came up. It gets what `current()` gives |
| `draw(kit, data, dt)` | Every frame. `kit` is a separate canvas and `data` is `obsidian.ui.data` |
| `exit()` | The screen went away |
| `mute` | Not a function: `true` for this screen, or a list of ids, held only while it's up |

The handlers of a session mod stop, and the mutes of that mod lift, while the lobby isn't cleared. Those of an expansion keep running outside the content. `off(id)` takes the handlers of the calling mod away.

### obsidian.ui.mute(id), unmute(id)

Stops a part of the game drawing, until `unmute`. The logic of that part keeps running.

| Id | What goes |
|---|---|
| The name of a menu | That menu: `"loadout"`, `"armory"`, `"options"`, `"mission_briefing"`, and so on |
| `"hud"` | Every part below |
| `"hud.panels"` | The player panels |
| `"hud.map"` | The map |
| `"hud.stratagems"` | The stratagem list and cooldowns |
| `"hud.names"` | Names over players |
| `"hud.world"` | Labels on things in the world |
| `"hud.prompts"` | Prompts on things to use |
| `"hud.dialogue"` | Dialogue |
| `"chat"`, `"hints"`, `"samples"`, `"news"`, `"save_icon"` | Each of those |
| `"mission_end"` | The steps of the end screens |

Parts nest: muting `"hud.panels"` leaves the map, until you mute that too. The 3D consoles on the ship, the galaxy map and the intro film aren't drawn this way, so they stay. The campaign screen draws a few frames directly, and those stay too.

### obsidian.ui.scope(id, target, method)

Adds a custom part. `mute(id)` then stops whatever that function of the game draws: `target` is a table or the name of a global one. It changes nothing else.

### obsidian.ui.data

Copies of what the game knows, so writing to one changes nothing. Each answers `nil` where it doesn't apply. `player` is a slot from 1 to 4, and `nil` means the player at this keyboard.

| Function | Gives |
|---|---|
| `players()` | `name`, `title`, `rank_icon`, `xp`, `color`, `index`, `slot`, `is_local`, `pad`, for each player |
| `loadout(player)` | On the Loadout screen: `items`, `available`, `selection`, `submenu`, `slot`, `ready`, `random`, `loadout`, `presets`, `new_unlocks`, `upgrades` |
| `hud(player)` | `state`, `health`, `shield`, `inventory`, `character`, `stratagems`, `cooldowns`, `respawn`. The tables use the field names of the game |
| `map(player)` | `zoom`, `offset`, `size`, `pings`, `markers` |
| `mission()` | `objectives`, `enemy`, `difficulty`, `environment`, `type`, `seed`, `game_over`, `exit` |
| `campaign()` | `status`, `statistics`, `season`. It's large, so ask once |
| `mission_end()` | `sub`, and `values` with every plain field of the state |

### obsidian.ui.canvas(id, opts)

A drawing kit, for code that isn't a `draw` handler. `opts.layer` runs from 0 to 875, and `opts.over_fades` keeps it at full strength while the game fades to black. By default a kit fades with the game.

Coordinates are pixels from the bottom left, or the design space of the mod after `obsidian.ui.frame`. Colors are `{ r, g, b }`, `{ r, g, b, a }`, `"#rrggbb"` or `"#rrggbbaa"`.

| Method | Draws |
|---|---|
| `kit:rect(x, y, w, h, color)` | A rectangle |
| `kit:poly(points, color)` | A convex shape, from `{ x1, y1, x2, y2, ... }` |
| `kit:outline(points, color, width)` | The edge of a shape, inside it |
| `kit:panel{ x, y, w, h, cut, corners, fill, line, line_width }` | A panel with corners cut at 45 degrees. `corners` is any of `"tl tr br bl"`, by default `"tr bl"` |
| `kit:hex(x, y, r, color, icon)` | A hexagon around a point, with an icon in it if you give one |
| `kit:glyph(name, x, y, size, color)` | One of the icons drawn from shapes, from the bottom left corner. `obsidian.ui.glyphs()` lists the names |
| `kit:text(text, x, y, opts)` | Text. `y` is the baseline and a line break starts a new line. `opts`: `font`, `size`, `tracking`, `align`, `color`, `leading` |
| `kit:measure(text, opts)` | Width and height of a text, tracking included. Draws nothing |
| `kit:point(x, y)` | A point in pixels, such as the mouse position, in the coordinates of this kit |
| `kit:icon(name, x, y, w, h, color, uv)` | One of the game images. `uv` is `{ u0, v0, u1, v1 }`. `obsidian.ui.icons()` lists the names |
| `kit:art(name, x, y, w, h, color, uv)` | An image from a material the mod ships, added with `obsidian.ui.art`. `uv` is `{ u0, v0, u1, v1 }`, and `u0` above `u1` mirrors it |
| `kit:band(x, y, w, h, color)` | The player band of the game, with the band gradient, tinted |
| `kit:bar(x, y, w, h, value, color, back)` | A bar filled from 0 to 1, over `back` |
| `kit:clip(x, y, w, h, fn)` | Runs `fn(kit)` with drawing kept inside the rectangle |
| `kit:layer(n)`, `kit:opacity(a)` | The layer and strength of what comes next |

Glyphs need no art, so they keep working where `kit:icon` doesn't. The in-game menu is built from these same calls, so anything it draws, a mod can draw too.

Fonts: `"body"`, `"body_large"`, `"heading"`, `"heading_large"`, `"ui"`, `"ui_heading"`, `"chat"`, `"debug"`, `"debug_medium"`. `"ui"` and `"ui_heading"` are Barlow and Saira, the fonts of the in-game menu, and they carry Latin letters with accents. `"heading"` is Insignia, and `"chat"` is the one with Chinese, Japanese and Korean, which a text falls back to when the chosen font lacks a letter. `"debug"` needs nothing loaded, and every other font falls back to it until the game has loaded them.

A clip cuts shapes and icons exactly, and text at whole letters. The menu art of the Loadout screen isn't available: the material unloads between states, and a gui that lists one that isn't loaded can stop the game. Custom art needs the same check, so a canvas takes only the game images for now.

### obsidian.ui.font(id, opts)

Adds a custom font, for `kit:text`. `tools/bake_font.py` in the Obsidian repository turns a TrueType file into what the game reads:

```
pip install pillow uharfbuzz
python tools/bake_font.py --out res --base mymod/fonts title=MyFont.ttf
```

Then list each file in the `resources` of the manifest, in the boot bundle:

```json
"resources": [
  { "type": "font", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.font" },
  { "type": "texture", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.texture" },
  { "type": "material", "name": "mymod/fonts/fonts", "bundle": "9cf71054ea26dddf", "data": "res/fonts.material" }
]
```

Add it when the mod starts, and use it by id:

```lua
obsidian.ui.font("title", { path = "mymod/fonts/title", size = 32 })
kit:text("HELLDIVERS", 100, 200, { font = "title" })
```

`size` is the size a text gets when it doesn't ask for one. The answer is `false` for a font the deploy doesn't have, and a text naming it gets the debug font. It has to be in the boot bundle, because a gui that lists a material that isn't loaded can stop the game. Text stays sharp up to the size you baked, 48 by default, and a letter the font lacks comes from `"chat"`. Check the font license: a bake is a modified version, so a font with a reserved name can't keep that name.

### obsidian.ui.art(name, opts)

Adds a sub-material of a material the mod ships in the boot bundle, for `kit:art`. `opts.path` is the resource name of the material. The answer is `false` for a material the deploy doesn't have, or when you ask for a new material after the game has started drawing, so ask while the mod starts.

### obsidian.ui.list_with(path, with)

Makes every gui the game creates that lists the material `with` list `path` too, so the game can draw a sub-material of `path` where it draws the ones of `with`. `with` is `"hud/hud"` if you leave it out, the material that holds the rank and stratagem icons. `path` is a material the mod ships in the boot bundle, and the answer is `false` for one the deploy doesn't have. Call it while the mod starts, before the game makes the guis. Put many icons in one material and list that one.

### obsidian.ui.frame(w, h), safe()

`frame(1920, 1080)` makes the coordinates of this mod a design space, scaled and centred the way the game does it, from the window and UI scale of the player. `frame()` goes back to pixels. Call it once, even while the mod starts. The game leaves the space at the bottom of the window when the UI scale is below 1; `frame(1920, 1080, { centered = true })` keeps it in the middle.

`safe()` gives `{ x, y, w, h, scale }`, the area the game keeps the game menus in, in the coordinates of the mod.

### obsidian.ui.remap_hits(id, fn)

The game tests the mouse against where it draws. When the layout of the mod moves something, `fn(x, y, w, h)` gets each rectangle in pixels and answers where it is now, `nil` to leave it, or `false` for nowhere. `remap_hits(id)` stops it.

### obsidian.ui.prompt(name, pad)

The button on the device of a player, as text to draw. `name` is an action such as `"select"`, `"back"`, `"fire"`, `"reload"` or `"map"`, a token such as `"#mod_map"`, or one of the game text keys, which comes back with the buttons filled in. `pad` is the pad number from `data.players()`, and `nil` means the main player.

## Saved values {#storage}

### obsidian.storage.get(key, default), set(key, value), all()

Strings, numbers and booleans this mod keeps between launches. `set` with `nil` removes a key. The file is written at once, so don't call it every frame.

### obsidian.save_data.get(key, default), set(key, value), sandbox() {#save-data}

Strings, numbers and booleans this mod keeps with the current save, in a file beside the save. Each [Sandbox](wiki:saves) has a separate file, and the Official file travels with the save when Obsidian switches Official saves. Use it for what belongs to one character, such as a manual unlock, and `obsidian.storage` for what belongs to the install. `set` with `nil` removes a key, and the file is written at once. `sandbox()` is the id of the Sandbox of this launch, or `nil` on an Official save. Check `obsidian.capabilities.save_data` first: an older loader doesn't have it.

Values a player should be able to edit in the save editor go in the [`save_editor` key](wiki:manifest#save-editor) of the manifest.

## Version and features

`obsidian.version` is the API version, `1` today. A mod that needs a newer API sets `loader_api` in `obsidian.json`, and older loaders refuse to start it.

`obsidian.capabilities` is a read-only table of what the running loader supports, one entry per feature: `hook`, `on_require`, `on_frame`, `on_session`, `after_startup`, `settings`, `keybind`, `session`, `text`, `log`, `module`, `import`, `caller`, `info`, `menu`, `toast`, `notify`, `storage`, `save_data`, `ui`, `expansion`, `language`, `game_text`, `secondary_scopes` (also `parts`, the older name). Test a feature by name before using one that came later:

```lua
if obsidian.capabilities.on_frame then
  obsidian.on_frame(update)
end
```

## When a mod fails

- An error at startup switches the mod off for this session and writes the error to `loader.log`.
- Errors in hooks and callbacks are logged at most three times per place, so a broken frame function can't flood the log.
- A mod that keeps failing is switched off for the rest of the session.
- If the game crashes while a mod starts, the app names that mod at the next launch.
