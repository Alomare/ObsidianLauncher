---
title: Loader API
group: Making mods
summary: Every function of the obsidian table, version 1.
---

Obsidian Loader starts the `entry` script of each enabled mod in load order, once, when the game boots. Every script sees a global table named `obsidian`. Everything a mod registers through it runs protected: an error is logged and never stops the game.

## Hooks {#hooks}

### obsidian.hook(target, method, fn)

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

In a session mod, the hook calls only the original while the lobby isn't cleared ([Writing session mods](wiki:session-mods)).

### obsidian.wrap(target, method, fn)

The same as `obsidian.hook`, with the same safety if `fn` fails, except that it keeps running while the lobby isn't cleared. It's for a mod whose replaced files can't be switched off either, where a hook going off would leave one game half modded. Anything else should use `obsidian.hook`. Check `obsidian.capabilities.wrap` on an older loader.

### obsidian.on_require(name, fn)

Calls `fn(module)` when the game loads the script `name`, such as `"lua/settings/perk_settings"`. Use it to change a settings table right after the game builds it.

### obsidian.on_frame(fn)

Calls `fn` once per frame. Return `true` to stop the calls. A session mod's frame functions only run in a cleared lobby.

### obsidian.after_startup(fn)

Calls `fn` once every mod has started, so mods can work together without depending on load order.

## Settings and keys

### obsidian.settings.get(id)

The value of option `id` of this mod: the one the player chose in the app, or the default ([Options and keys](wiki:options-and-keys)).

### obsidian.keybind(id, fn)

Calls `fn` each time the key bound to `id` goes down. The key comes from the app, or from the default in `obsidian.json`.

## Multiplayer

### obsidian.session.cleared()

`true` while the calling mod may run: the session is cleared, and for a mod marked `every_player`, every other player runs it too ([Mods every player needs](wiki:session-mods#every-player)). Client mods rarely need it.

### obsidian.session.guard(t, free)

A view of table `t` to hand to other mods. The functions named in the list `free` work as usual. Every other function answers `false, "needs_session"` when the calling mod runs as a client mod, and `false, "not_cleared"` when the session isn't cleared ([Writing a library](wiki:session-mods#libraries)).

### obsidian.session.members()

A list of the other players in the lobby. Each entry has `peer`, `obsidian` (whether that player runs Obsidian), `modset` and `consent`.

To run only when every player has your mod, set [`every_player`](wiki:session-mods#every-player) in the manifest instead of comparing these yourself.

### obsidian.session.forfeit_progress(), release_progress(key) and hold_progress()

These do nothing. A player's own save is theirs, so Obsidian holds none of a mission's progress back, whatever the mods do. They stay so mods that call them keep working. Only Galactic War and leaderboard reports stay blocked while session content is deployed ([The Galactic War](wiki:galactic-war)). To play without a mission counting, a player switches on **Freeze my account's progression** in [Guardrails](wiki:guardrails).

### obsidian.guardrails()

What the player chose in **Guardrails** for this launch, as `{ consent, block_achievements, freeze_progress }`. A mod can read them, for example to tell the player their account is frozen. Only the player can change them, and the loader enforces them under every mod ([Guardrails](wiki:guardrails)).

### obsidian.on_session(fn)

Calls `fn(on)` whenever the loader switches this mod on or off, so the mod can undo what other players would see. A mod marked `every_player` also hears when a player who doesn't run it comes or goes.

## Text and logs

### obsidian.text(key, ...)

The text `key` from the `locales` folder of the mod, in the language of the game, with English as the fallback. Extra arguments fill `%s` and `%d` placeholders.

### obsidian.log(...)

Writes a line to the log of this mod, in `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager\logs`.

## Other mods and more files {#other-mods}

### obsidian.module(name)

Loads another Lua file of this mod and returns what it returns. The file has to be listed in `modules` in `obsidian.json`, and `name` is its path without `.lua`, such as `"lib/parts"`.

### obsidian.import(id)

What the `entry` script of another mod returned, or `nil` when that mod isn't running. List the mod in `requires`, so it starts first ([Obsidian API](wiki:obsidian-api)).

### obsidian.scope()

How the calling mod runs in this deploy: `"client"` or `"session"`. A library gets one of the two ([Writing a library](wiki:session-mods#libraries)).

### obsidian.caller()

The id of the mod whose code is running, so a shared mod can record who asked for something.

## In-game menu {#menu}

### obsidian.menu.add(title, build, opts)

Adds a page to the list down the left of the in-game menu. `title` is text, or a function that returns it. `build(page)` runs each time the page opens. `opts.icon` is one of `obsidian.ui.glyphs()`:

```lua
obsidian.menu.add("My mod", function(page)
  page:header("Tuning")
  page:toggle("Enabled", { get = function() return on end, set = function(v) on = v end })
  page:number("Power", { min = 1, max = 10, step = 1, get = get_power, set = set_power })
  page:choice("Mode", { choices = { { id = "a", label = "First" } }, get = get_mode, set = set_mode })
  page:action("Do it", function() return "Done" end)
  page:input("Name", function(text) end)
  page:page("More", function(sub) sub:label("Deeper") end)
end, { icon = "player" })
```

Any row takes `help`, a text shown while it's selected, and `enabled`, a function that returns `false` and a reason. A string returned by an action shows as a toast. An action can call `page:refresh()` to build the page again. A failing row shows the error and doesn't count against your mod.

A `toggle`, `number` or `choice` row takes `default`, the value it starts at. It then gets a reset button at the end of the row, lit once the value has moved, and R does the same from the keyboard. Where writing a value isn't the way back, give `reset`, a function that undoes it, and `changed`, one that says whether there's anything to undo.

A session mod's pages aren't listed, and close, while the lobby isn't cleared.

### obsidian.menu.open(), close(), toggle(), is_open()

Nothing opens the menu by itself, so bind a key to it:

```lua
obsidian.keybind("open", function() obsidian.menu.toggle() end)
```

### obsidian.menu.overlay(fn)

Calls `fn(draw, width, height)` every frame, to put something on screen outside the menu. `draw.text(text, x, y, color)` and `draw.rect(x, y, w, h, color, alpha)` work in pixels from the bottom left. Colors are `{ r, g, b }`.

### obsidian.toast(text, seconds)

Shows a short message at the top left of the screen, or beside the in-game menu while it's open.

### obsidian.notify(text, seconds) {#obsidian-notify-text-seconds}

Shows your words in the game's own hint box, with its icon and sound, even with the game's hints turned off. `seconds` is 7 if you leave it out. Where the game has no hint box, such as the main menu, it shows as a toast instead.

The box shows one message at a time, so use it for what's rare and matters, and `obsidian.toast` for quick feedback such as a key press. A text already waiting isn't added again, and a mod can have 3 waiting: past that, `notify` returns `false` and `"queue_full"`. Obsidian's own messages go first.

## Replacing the game's screens {#ui}

`obsidian.ui` is for a mod that redraws the game's menus and HUD. Any mod can use it, of any scope, with nothing in `requires`, so a reskin stays a client mod and keeps the Galactic War and matchmaking. Nothing here runs until a mod asks for it.

The idea is to keep the game's logic and input, stop its drawing, read its data and draw on top. The game's update and draw functions always run, because keys, mouse and network code live inside them.

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

If a screen's code fails, that screen stops for the session, the game draws its own again, and the error goes in `loader.log`.

### obsidian.ui.current()

Where the player is: `{ state, screen, sub }`. `screen` is the id the rest of this section uses: a menu by its name (`"loadout"`, `"armory_profile"`, `"main"`), `"title"` and `"ship"` with no menu open, `"hud"` in a mission, or the state's own name, such as `"mission_end"`. `sub` is the step inside it, when it has one.

### obsidian.ui.on(id, handlers), off(id)

Runs code while a screen is up. `id` is a value of `screen`, or `"*"` for all of them.

| Handler | When |
|---|---|
| `enter(screen)` | The screen came up. It gets what `current()` gives |
| `draw(kit, data, dt)` | Every frame. `kit` is a canvas of its own and `data` is `obsidian.ui.data` |
| `exit()` | The screen went away |
| `mute` | Not a function: `true` for this screen, or a list of ids, held only while it's up |

A session mod's handlers stop, and its mutes lift, while the lobby isn't cleared. `off(id)` takes this mod's handlers away.

### obsidian.ui.mute(id), unmute(id)

Stops a part of the game drawing, until `unmute`. Its logic keeps running.

| Id | What goes |
|---|---|
| A menu's name | That menu: `"loadout"`, `"armory"`, `"options"`, `"mission_briefing"`, and so on |
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

Adds your own part. `mute(id)` then stops whatever that function of the game draws: `target` is a table or the name of a global one. It changes nothing else.

### obsidian.ui.data

Copies of what the game knows, so writing to one changes nothing. Each answers `nil` where it doesn't apply. `player` is a slot from 1 to 4, and `nil` means the player at this keyboard.

| Function | Gives |
|---|---|
| `players()` | `name`, `title`, `rank_icon`, `xp`, `color`, `index`, `slot`, `is_local`, `pad`, for each player |
| `loadout(player)` | On the Loadout screen: `items`, `available`, `selection`, `submenu`, `slot`, `ready`, `random`, `loadout`, `presets`, `new_unlocks`, `upgrades` |
| `hud(player)` | `state`, `health`, `shield`, `inventory`, `character`, `stratagems`, `cooldowns`, `respawn`. The tables use the game's own field names |
| `map(player)` | `zoom`, `offset`, `size`, `pings`, `markers` |
| `mission()` | `objectives`, `enemy`, `difficulty`, `environment`, `type`, `seed`, `game_over`, `exit` |
| `campaign()` | `status`, `statistics`, `season`. It's large, so ask once |
| `mission_end()` | `sub`, and `values` with every plain field of the state |

### obsidian.ui.canvas(id, opts)

A drawing kit, for code that isn't a `draw` handler. `opts.layer` runs from 0 to 875, and `opts.over_fades` keeps it at full strength while the game fades to black. By default a kit fades with the game.

Coordinates are pixels from the bottom left, or your design space after `obsidian.ui.frame`. Colors are `{ r, g, b }`, `{ r, g, b, a }`, `"#rrggbb"` or `"#rrggbbaa"`.

| Method | Draws |
|---|---|
| `kit:rect(x, y, w, h, color)` | A rectangle |
| `kit:poly(points, color)` | A convex shape, from `{ x1, y1, x2, y2, ... }` |
| `kit:outline(points, color, width)` | The edge of a shape, inside it |
| `kit:panel{ x, y, w, h, cut, corners, fill, line, line_width }` | A panel with corners cut at 45 degrees. `corners` is any of `"tl tr br bl"`, by default `"tr bl"` |
| `kit:hex(x, y, r, color, icon)` | A hexagon around a point, with an icon in it if you give one |
| `kit:glyph(name, x, y, size, color)` | One of the icons drawn from shapes, from its bottom left corner. `obsidian.ui.glyphs()` lists the names |
| `kit:text(text, x, y, opts)` | Text. `y` is the baseline and a line break starts a new line. `opts`: `font`, `size`, `tracking`, `align`, `color`, `leading` |
| `kit:measure(text, opts)` | Width and height of a text, tracking included. Draws nothing |
| `kit:point(x, y)` | A point in pixels, such as the mouse's, in this kit's coordinates |
| `kit:icon(name, x, y, w, h, color, uv)` | One of the game's images. `uv` is `{ u0, v0, u1, v1 }`. `obsidian.ui.icons()` lists the names |
| `kit:art(name, x, y, w, h, color, uv)` | An image from a material your mod ships, added with `obsidian.ui.art`. `uv` is `{ u0, v0, u1, v1 }`, and `u0` above `u1` mirrors it |
| `kit:band(x, y, w, h, color)` | The game's player band: its gradient, tinted |
| `kit:bar(x, y, w, h, value, color, back)` | A bar filled from 0 to 1, over `back` |
| `kit:clip(x, y, w, h, fn)` | Runs `fn(kit)` with drawing kept inside the rectangle |
| `kit:layer(n)`, `kit:opacity(a)` | The layer and strength of what comes next |

Glyphs need no art, so they keep working where `kit:icon` doesn't. The in-game menu is built from these same calls, so anything it draws, a mod can draw too.

Fonts: `"body"`, `"body_large"`, `"heading"`, `"heading_large"`, `"ui"`, `"ui_heading"`, `"chat"`, `"debug"`, `"debug_medium"`. `"ui"` and `"ui_heading"` are Barlow and Saira, the in-game menu's fonts, and they carry Latin letters with accents. `"heading"` is Insignia, and `"chat"` is the one with Chinese, Japanese and Korean, which a text falls back to when its font lacks a letter. `"debug"` needs nothing loaded, and every other font falls back to it until the game has loaded them.

A clip cuts shapes and icons exactly, and text at whole letters. The Loadout screen's menu art isn't available: its material unloads between states, and a gui that lists one that isn't loaded can stop the game. Your own art needs the same check, so a canvas takes only the game's images for now.

### obsidian.ui.font(id, opts)

Adds a font of your own, for `kit:text`. `tools/bake_font.py` in Obsidian's repository turns a TrueType file into what the game reads:

```
pip install pillow uharfbuzz
python tools/bake_font.py --out res --base mymod/fonts title=MyFont.ttf
```

Then list each file in your manifest's `resources`, in the boot bundle:

```json
"resources": [
  { "type": "font", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.font" },
  { "type": "texture", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.texture" },
  { "type": "material", "name": "mymod/fonts/fonts", "bundle": "9cf71054ea26dddf", "data": "res/fonts.material" }
]
```

Add it when your mod starts, and use it by its id:

```lua
obsidian.ui.font("title", { path = "mymod/fonts/title", size = 32 })
kit:text("HELLDIVERS", 100, 200, { font = "title" })
```

`size` is the size a text gets when it doesn't ask for one. The answer is `false` for a font the deploy doesn't have, and a text naming it gets the debug font. It has to be in the boot bundle, because a gui that lists a material that isn't loaded can stop the game. Text stays sharp up to the size you baked, 48 by default, and a letter your font lacks comes from `"chat"`. Check your font's license: a bake is a modified version, so a font with a reserved name can't keep that name.

### obsidian.ui.art(name, opts)

Adds a sub-material of a material your mod ships in the boot bundle, for `kit:art`. `opts.path` is the material's resource name. The answer is `false` for a material the deploy doesn't have.

### obsidian.ui.frame(w, h), safe()

`frame(1920, 1080)` makes this mod's coordinates a design space, scaled and centred the way the game does it, from the player's window and UI scale. `frame()` goes back to pixels. Call it once, even while the mod starts. The game leaves the space at the bottom of the window when the UI scale is below 1; `frame(1920, 1080, { centered = true })` keeps it in the middle.

`safe()` gives `{ x, y, w, h, scale }`, the area the game keeps its own menus in, in your coordinates.

### obsidian.ui.remap_hits(id, fn)

The game tests the mouse against where it draws. When your layout moves something, `fn(x, y, w, h)` gets each rectangle in pixels and answers where it is now, `nil` to leave it, or `false` for nowhere. `remap_hits(id)` stops it.

### obsidian.ui.prompt(name, pad)

The button for a player's device, as text to draw. `name` is an action such as `"select"`, `"back"`, `"fire"`, `"reload"` or `"map"`, a token such as `"#mod_map"`, or one of the game's text keys, which comes back with its buttons filled in. `pad` is the pad number from `data.players()`, and `nil` means the main player.

## Saved values {#storage}

### obsidian.storage.get(key, default), set(key, value), all()

Strings, numbers and booleans this mod keeps between launches. `set` with `nil` removes a key. The file is written at once, so don't call it every frame.

## Version and features

`obsidian.version` is the API version, `1` today. A mod that needs a newer API sets `loader_api` in `obsidian.json`, and older loaders refuse to start it.

`obsidian.capabilities` is a read-only table of what the running loader supports, one entry per feature: `hook`, `on_require`, `on_frame`, `on_session`, `after_startup`, `settings`, `keybind`, `session`, `text`, `log`, `module`, `import`, `caller`, `menu`, `toast`, `notify`, `storage`, `ui`. Test a feature by name before using one that came later:

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
