---
title: Options and keys
group: Making mods
summary: Settings players change in the app, and keys they can rebind, read from Lua.
---

Options show in the panel on the right when a player selects the mod. Obsidian checks every value against the declaration, so the mod never receives a value outside the declared range. Values belong to the profile, so two profiles can run the same mod with different settings.

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

### Options of a part

In a mod with [parts](wiki:manifest#parts), `"part": "<part id>"` puts an option in that part. An option of a client part counts as `"scope": "client"`, and an option of a part that is off doesn't count at all. An option without a part follows the strictest part that is on. The toggle a part names in `option` switches that part, so changing the toggle needs a deploy.

## Option groups

`option_groups` sorts the options under headings that fold. Each option names the heading with `group`:

```json
"option_groups": [
  { "id": "idle", "label": { "en": "Idle crosshair" }, "description": { "en": "While not aiming." } },
  { "id": "aim", "label": { "en": "Aiming" }, "collapsed": true }
],
"options": [
  { "id": "idle_shape", "type": "choice", "default": "cross", "choices": ["cross", "dot"], "group": "idle",
    "label": { "en": "Shape" } }
]
```

Options without a group come first, then each group in the order declared. `collapsed` starts a group folded. The app remembers which groups the player opened, for each mod. `visible_when` works inside groups as anywhere else. A `group` that names no group is refused on import. Older Obsidian ignores both fields and lists the options as before.

## Option previews

`option_previews` shows a picture of the values, drawn from an SVG file in the mod folder and redrawn as the player changes them:

```json
"option_previews": [
  { "id": "idle_preview", "group": "idle", "template": "preview/idle.svg", "width": 240, "height": 140,
    "label": { "en": "Preview" } }
]
```

The preview shows at the top of the group it names, or at the top of the Options tab. The file is SVG, at most 64 KB, and is checked on import. Inside it:

- `{{expr}}` in an attribute value or in text becomes the value of the expression.
- `data-if="expr"` on any element removes the element while the expression is false.

An expression uses numbers, `'text'`, option ids (a number, `true` or `false`, a choice id or a `#rrggbb` color), `+ - * / %`, `== != < > <= >=`, `&& || !`, `a ? b : c`, parentheses and `min`, `max`, `abs`, `round`, `floor`, `ceil` and `clamp(x, lo, hi)`. `+` joins text. An expression with a mistake gives nothing, and an element whose `data-if` has one is removed.

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="240" height="140">
  <rect width="240" height="140" fill="#1b1d1f"/>
  <g data-if="idle_shape == 'cross'" stroke="{{idle_color}}" stroke-width="{{idle_thickness}}">
    <line x1="{{120 - idle_size}}" y1="70" x2="{{120 + idle_size}}" y2="70"/>
    <line x1="120" y1="{{70 - idle_size}}" x2="120" y2="{{70 + idle_size}}"/>
  </g>
  <circle data-if="idle_shape == 'dot'" cx="120" cy="70" r="{{idle_size / 2}}" fill="{{idle_color}}"/>
</svg>
```

The picture shows as an image: scripts, `foreignObject`, `on...` attributes and links outside the file are taken out.

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

The Keys tab has a column for the mouse and keyboard and one for a controller. When an action also has a controller input, declare that input as a `choice` option and name the option in the `pad` of the keybind:

```json
"options": [ { "id": "pad_toggle", "type": "choice", "default": "thumbs",
               "choices": [ { "id": "thumbs", "label": { "en": "Both sticks pressed in" } }, { "id": "off", "label": { "en": "Off" } } ] } ],
"keybinds": [ { "id": "toggle", "default": "f8", "pad": "pad_toggle", "label": { "en": "Turn the compass on or off" } } ]
```

The app then shows the option in the row of the keybind, under the controller column, and leaves it out of the Options tab. The mod reads the option like any other and watches the pad itself. A keybind with no `pad` shows a dash in that column. Obsidian versions without the controller column ignore `pad` and list the option under Options.
