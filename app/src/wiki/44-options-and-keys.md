---
title: Options and keys
group: Making mods
summary: Settings players change in the app, and keys they can rebind, read from Lua.
---

Options show in the panel on the right when a player selects your mod. Obsidian checks every value against what you declared, so the mod never receives a value outside the range you set. Values belong to the profile, so two profiles can run the same mod with different settings.

## Declaring options

Each option has an `id` (lowercase letters, digits and `_`), a `type`, a `default` and a `label`. A `description` adds the text players see when they hover the option.

| Type | Value | Extra fields |
|---|---|---|
| `toggle` | `true` or `false` | |
| `number` | a number | `min`, `max`, `step` |
| `choice` | one of the listed ids | `choices`: a list of ids, or of `{ "id": "…", "label": { … } }` |
| `key` | a key name such as `f8` | |
| `color` | `#rrggbb` | |

```json
"options": [
  { "id": "enabled", "type": "toggle", "default": true, "label": { "en": "Show the compass" } },
  { "id": "style", "type": "choice", "default": "small",
    "choices": [ { "id": "small", "label": { "en": "Small" } }, { "id": "large", "label": { "en": "Large" } } ],
    "label": { "en": "Size" }, "visible_when": { "enabled": true } },
  { "id": "tint", "type": "color", "default": "#f0b429", "label": { "en": "Color" }, "scope": "client" }
]
```

### Showing options only when they matter

`visible_when` hides an option until other options have the listed values. Above, **Size** only shows while **Show the compass** is on.

### Personal options in session mods

The option values of a session mod or an expansion count when Obsidian compares two players' mods, because a balance slider changes the shared game. Mark an option `"scope": "client"` when the value only changes what this player sees, such as a color or a camera distance. Players can then pick different values and still match ([Playing online](wiki:multiplayer)).

## Reading options in Lua

```lua
local get = obsidian.settings.get

if get("enabled") then
  local size = get("style")   -- "small" or "large"
end
```

`obsidian.settings.get` returns the value the player chose, or the default. A change in the app applies the next time the game starts, with no deploy.

## Options that pick files

A resource can depend on an option with `when`. The resource only goes into the game while the options have those values:

```json
"resources": [
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_small.lua", "when": { "style": "small" } },
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_large.lua", "when": { "style": "large" } }
]
```

Changing such an option needs a new deploy, and the app turns the Deploy button green to say so.

## Keys

Declare each key the mod listens for, with a default:

```json
"keybinds": [ { "id": "toggle", "default": "f8", "label": { "en": "Turn the compass on or off" } } ]
```

Then react to it in Lua:

```lua
obsidian.keybind("toggle", function()
  obsidian.log("toggle pressed")
end)
```

The function runs once each time the key goes down. Players rebind keys in the app, which is where clashes between mods get sorted out. Key names are the ones the game uses, in lower case: `f8`, `k`, `insert`.

### Controller input

The Keys tab has a column for the mouse and keyboard and one for a controller. When an action also has a controller input, declare that input as a `choice` option and name the option in the keybind's `pad`:

```json
"options": [ { "id": "pad_toggle", "type": "choice", "default": "thumbs",
               "choices": [ { "id": "thumbs", "label": { "en": "Both sticks pressed in" } }, { "id": "off", "label": { "en": "Off" } } ] } ],
"keybinds": [ { "id": "toggle", "default": "f8", "pad": "pad_toggle", "label": { "en": "Turn the compass on or off" } } ]
```

The app then shows the option in the keybind's row, under the controller column, and leaves it out of the Options tab. The mod reads the option like any other and watches the pad itself. A keybind with no `pad` shows a dash in that column. Obsidian versions without the controller column ignore `pad` and list the option under Options.
