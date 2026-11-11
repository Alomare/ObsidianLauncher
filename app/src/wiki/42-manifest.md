---
title: The obsidian.json file
group: Making mods
summary: Every field of the file that describes a mod.
---

`obsidian.json` sits at the top of the mod folder, or at the top of the `.zip`. Obsidian checks it on import and says which field is wrong when something doesn't fit.

## A full example

```json
{
  "obsidian": 1,
  "id": "yourname.better-camera",
  "version": "1.2.0",
  "name": { "en": "Better Camera", "pt-BR": "Câmera Melhor" },
  "description": { "en": "A closer camera with a toggle key." },
  "authors": ["Author Name"],
  "image": "image.png",
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1,
  "options": [
    { "id": "distance", "type": "number", "default": 5, "min": 1, "max": 12, "step": 0.5,
      "label": { "en": "Camera distance" } }
  ],
  "keybinds": [
    { "id": "toggle", "default": "f8", "label": { "en": "Toggle the camera" } }
  ],
  "resources": []
}
```

## Fields

| Field | Required | What it is |
|---|---|---|
| `obsidian` | yes | The format version. Always `1` for now. |
| `id` | yes | Permanent id: 1 to 64 of `a-z`, `0-9`, `.`, `_` and `-`, starting with a letter or digit. Use `yourname.mod-name`. |
| `version` | yes | A version number such as `1.2.0` (semantic versioning). A higher version replaces a lower one on import. |
| `name` | yes | The mod name by language. English (`en`) is required. |
| `description` | no | One or two sentences by language, shown under the name. |
| `authors` | no | A list of names. |
| `image` | no | A square PNG, JPG or WebP inside the mod folder, up to 4 MB. |
| `source` | no | The web address of the mod, shown in the details. A GitHub repository such as `https://github.com/author/mod` also lets players update the mod from the app ([Sharing a mod](wiki:sharing#updates)). |
| `scope` | no | `client`, `session`, `expansion` or `library` ([Scopes](wiki:scopes)). Missing or unknown makes the mod unspecified, which runs as a client mod, or as a session mod with `every_player`. |
| `every_player` | no | `true` for a mod that only works when every player in the lobby runs it ([Mods every player needs](wiki:session-mods#every-player)). Not for client mods or expansions. |
| `parts` | no | Pieces of the mod with a scope of their own ([Parts](#parts)). Not for libraries or expansions. |
| `entry` | no | The Lua script the loader starts. Mods that only replace game files leave it out. |
| `modules` | no | More `.lua` files beside `entry`, which the mod loads with `obsidian.module`. Paths inside the mod folder. |
| `requires` | no | Ids of mods this one needs. Switching the mod on switches them on, and they start first. Players see them in the Dependencies tab of the mod. A deploy stops with a message if one is missing. |
| `load` | no | Where the mod asks to sit in the load order: `"first"`, `"last"`, or an object with more requests ([Load order requests](#load-order-requests)). |
| `load_after` | no | Ids of mods this one asks to load after. The same as `after` inside `load`, kept for mods written before it. |
| `loader_api` | no | The version of the [Loader API](wiki:loader-api) the mod needs. `1` when left out. |
| `min_obsidian` | no | The oldest Obsidian the mod works with, such as `"1.1.0"`. An older one refuses the mod and asks the player to update. Obsidian before 1.1 doesn't read it. |
| `options` | no | Settings players change in the app ([Options and keys](wiki:options-and-keys)). |
| `option_groups` | no | Headings that fold, to sort the options under ([Option groups](wiki:options-and-keys#option-groups)). |
| `option_previews` | no | Pictures of the option values, from SVG templates ([Option previews](wiki:options-and-keys#option-previews)). |
| `keybinds` | no | Keys players can rebind in the app. The `pad` of a keybind names the choice option with the controller input for the same action ([Controller input](wiki:options-and-keys#controller-input)). |
| `resources` | no | Game files the mod replaces or adds ([Replacing game files](wiki:resources)). |
| `save_editor` | no | What the mod shows on the Mods tab of the [save editor](wiki:saves) ([The save editor](#save-editor)). |

## Load order requests

`load` takes `"first"` or `"last"` alone, or an object with any of these fields:

```json
"load": {
  "position": "last",
  "before": ["author.other-mod"],
  "after": ["author.base-mod"],
  "after_patches": true
}
```

| Field | What it asks for |
|---|---|
| `position` | `"first"` for the top of the load order, `"last"` for the end. |
| `before` | Ids of mods this one loads before. |
| `after` | Ids of mods this one loads after, so this mod wins the files both replace. |
| `before_patches` | `true` to load before every mod that replaces game files (has `resources`). |
| `after_patches` | `true` to load after every mod that replaces game files. |

A new mod takes the place it asks for when it joins a profile. After that the order the player set stands until the player clicks **Sort Mods** on the Mods tab ([Sort Mods](wiki:load-order#sort-mods)), which moves only the mods that need it. Ids of mods the player doesn't have are skipped, and Sort Mods says so.

Ask only for what the mod needs. Two mods that both ask to load last, or that ask to load after each other, can't both have their way: Sort Mods leaves them where they are and tells the player to order them by hand.

Obsidian checks `load` on import: a position other than `first` or `last`, an unknown field, the id of the mod itself, or the same id in `before` and `after` is refused. Obsidian 1.3.1 and earlier read only `"last"`, and list a mod with any other `load` as one that can't be read.

## Parts

`parts` splits a session mod into pieces, each `client` or `session`, so the personal pieces keep running in a lobby that isn't cleared ([Mods with parts](wiki:scopes#mods-with-parts)).

```json
"scope": "session",
"parts": [
  { "id": "view", "scope": "client",
    "name": { "en": "Camera and view" }, "description": { "en": "Keeps working in any lobby." } },
  { "id": "weapon_range", "scope": "session", "option": "max_weapon_range",
    "name": { "en": "Max weapon range" } }
]
```

| Field | What it is |
|---|---|
| `id` | Lowercase letters, digits and `_`, as an option id. |
| `scope` | `client` or `session`. |
| `name`, `description` | Text by language, shown in the details of the mod and in the in-game notice. |
| `option` | A toggle option of the mod. The part is on only while that option is on, and changing it needs a deploy. |

Code, options and files name their part: `opts.part` in the [Loader API](wiki:loader-api#parts), `part` on an [option](wiki:options-and-keys) and on a [resource](wiki:resources#parts). Anything without a part follows the strictest part that is on. A part that is off runs no code, deploys no file and doesn't count for what players compare.

The top-level `scope` stays the strictest one: `session` when any part is. An older Obsidian reads only that, and runs the whole mod as a session mod. Obsidian refuses a part id that isn't unique, a scope other than `client` or `session`, an `option` that isn't a toggle, and a `part` that names no part.

## Text in several languages

Every field that players read is an object with one entry per language: `en`, `pt-BR`, `zh-CN` for the app, and `en`, `fr`, `es`, `pt-BR` for text the game shows. English is always required and fills in for any language you leave out.

Text the mod shows in game goes in a `locales` folder beside `obsidian.json`, one file per language (`locales/en.json`, `locales/fr.json` and so on), each a simple object of keys and text. The mod reads them with `obsidian.text`.

## The save editor {#save-editor}

`save_editor` adds a section for the mod to the Mods tab of the [save editor](wiki:saves), for Sandbox saves. The values live in the data the mod reads with [`obsidian.save_data`](wiki:loader-api#save-data), under the mod id.

```json
"save_editor": {
  "items": [
    { "type": "stratagem", "name": "my_strike", "label": { "en": "Orbital Strike" }, "unlock": "manual" },
    { "type": "weapon", "name": "my_rifle", "label": { "en": "Long Rifle" }, "unlock": { "rank": 10 } }
  ],
  "fields": [
    { "id": "difficulty", "type": "choice", "label": { "en": "Difficulty" }, "choices": ["easy", "hard"] },
    { "id": "boost", "type": "number", "label": { "en": "Boost" }, "min": 0, "max": 5, "when": { "difficulty": "hard" } },
    { "id": "enabled", "type": "toggle", "label": { "en": "Enabled" } }
  ]
}
```

`items` lists what the mod adds to the game. `type` is `stratagem`, `weapon` or `perk`, and `unlock` says how a save gets the item: `"manual"` shows a switch, `"free"` and `"base"` mean every save has it, and `{ "rank": n }` means it comes with that rank.

`fields` are values the player can change. `type` is `number` (with `min` and `max`), `toggle` or `choice` (with `choices`, each an id or an object with `id` and `label`). `when` greys the field out unless the named fields hold the named values.

While the mod is off, the section greys out and the data stays in the save. If the mod leaves the library, the data is listed under the mod id for the player to delete. `"type": "tree"` is reserved for later.

## Fields Obsidian doesn't know

Unknown fields are kept and ignored, so a mod written for a later version of Obsidian still imports.
