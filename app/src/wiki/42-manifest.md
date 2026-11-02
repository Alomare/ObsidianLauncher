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
  "authors": ["Your name"],
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
| `source` | no | The mod's address, shown in its details. A GitHub repository such as `https://github.com/you/mod` also lets players update the mod from the app ([Sharing a mod](wiki:sharing#updates)). |
| `scope` | no | `client`, `session` or `library` ([Scopes](wiki:scopes)). Missing or unknown makes the mod unspecified, which runs as a client mod, or as a session mod with `every_player`. |
| `every_player` | no | `true` for a mod that only works when every player in the lobby runs it ([Mods every player needs](wiki:session-mods#every-player)). Not for client mods. |
| `entry` | no | The Lua script the loader starts. Mods that only replace game files leave it out. |
| `modules` | no | More `.lua` files beside `entry`, which the mod loads with `obsidian.module`. Paths inside the mod folder. |
| `requires` | no | Ids of mods this one needs. Switching the mod on switches them on, and they start first. Players see them in the mod's Dependencies tab. A deploy stops with a message if one is missing. |
| `loader_api` | no | The version of the [Loader API](wiki:loader-api) the mod needs. `1` when left out. |
| `options` | no | Settings players change in the app ([Options and keys](wiki:options-and-keys)). |
| `keybinds` | no | Keys players can rebind in the app. |
| `resources` | no | Game files the mod replaces or adds ([Replacing game files](wiki:resources)). |

## Text in several languages

Every field that players read is an object with one entry per language: `en`, `pt-BR`, `zh-CN` for the app, and `en`, `fr`, `es`, `pt-BR` for text the game shows. English is always required and fills in for any language you leave out.

Text the mod shows in game goes in a `locales` folder beside `obsidian.json`, one file per language (`locales/en.json`, `locales/fr.json` and so on), each a simple object of keys and text. The mod reads them with `obsidian.text`.

## Fields Obsidian doesn't know

Unknown fields are kept and ignored, so a mod written for a later version of Obsidian still imports.
