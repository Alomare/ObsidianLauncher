---
title: Replacing game files
group: Making mods
summary: Changing game scripts and other resources by name, without shipping whole bundles.
---

Besides running a script, a mod can replace resources of the game, or add new ones, through the `resources` list. Obsidian puts them into the patches at deploy and works out conflicts with other mods resource by resource.

## A changed game script

Copy the game script you want to change into your mod folder, edit it, and list it:

```json
"resources": [
  { "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" }
]
```

- `type` and `name` are the game path of the resource.
- `source` is plain Lua in your mod folder. Obsidian compiles it at deploy.
- `bytecode` can replace `source` for a file that is already compiled LuaJIT 2.0 bytecode. Obsidian checks the header and refuses anything else.

> [!TIP]
> A hook changes one function and leaves the rest of the script to the game, so two mods can change the same script without clashing. Prefer [hooks](wiki:loader-api#hooks) for code, and replace a whole script only when you have to.

## Other resources

Textures, units, materials and other types use `data`, the raw resource as the game stores it:

```json
{ "type": "texture", "name": "units/foo/bar", "data": "res/bar.texture" }
```

Older mods converted on import use this form. Making such files from scratch needs tools that don't exist for HD1 yet.

## Sound banks and videos {#streamed-resources}

Sound banks (`timpani_bank`) and videos (`bik`) keep most of their bytes in a bundle's `.stream` file. Give that part as `stream`, beside `data`:

```json
{ "type": "timpani_bank", "name": "#e709d0c5697fb286", "bundle": "0e2ec9da60a0f53a",
  "data": "res/menu.timpani_bank", "stream": "res/menu.stream" }
```

A resource the game has in several languages lists each one under `variants`, with the game's language flag in `lang`. A variant without `lang` is 0, the one every language without its own variant uses:

```json
{ "type": "timpani_bank", "name": "#f0b3f997d587fb77", "bundle": "f0b3f997d587fb77",
  "variants": [
    { "data": "res/voices.timpani_bank", "stream": "res/voices.stream" },
    { "lang": 256, "data": "res/voices.fr.timpani_bank", "stream": "res/voices.fr.stream" }
  ] }
```

| Language | `lang` |
|---|---|
| Every language without its own variant | 0 |
| Japanese | 1 |
| French | 256 |
| Spanish (Latin America) | 512 |
| Spanish (Spain) | 1024 |
| Portuguese (Brazil) | 2048 |
| Russian | 4096 |
| Italian | 8192 |
| German | 16384 |

At deploy, Obsidian writes the streamed parts into a `.stream` file beside the patch (`<bundle>.patch_0.stream`), in the order the patch lists them, as the game lays out its own. Sound changes only what you hear, so a mod of sound banks can be a [client mod](wiki:scopes).

## Fields

| Field | What it is |
|---|---|
| `type` | The resource type, such as `lua` or `texture`, or `#` plus 16 hex digits for a type with no known name |
| `name` | The resource name, such as `lua/settings/perk_settings`, or `#` plus 16 hex digits |
| `source` | Lua source to compile |
| `bytecode` | Compiled LuaJIT 2.0 bytecode |
| `data` | A raw resource of any other type |
| `stream` | With `data`: the part of the resource the game reads from the `.stream` file |
| `variants` | Instead of `data`: one entry per language, each with `lang`, `data` and an optional `stream` |
| `bundle` | The bundle the resource goes in, 16 hex digits. Only needed for a resource the game doesn't have yet |
| `flags` | A value bundles keep beside each resource; leave it out unless a converter set it |
| `when` | Options that must have these values for the resource to be used ([Options and keys](wiki:options-and-keys#options-that-pick-files)) |

## Limits

A replaced resource is static: it can't be switched off in the middle of a game. In a session mod, use resources for data the game reads at startup and hooks for behavior.
