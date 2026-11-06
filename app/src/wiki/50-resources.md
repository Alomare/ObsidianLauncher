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

## Single clips of a sound bank {#sound-bank-clips}

A whole bank can pass a gigabyte across its languages. A mod that changes a few sounds can ship only those clips under `clips`. At deploy, Obsidian puts them under new ids into a small bank of its own, beside the game's list of sounds (`timpani_master`, in the boot bundle), and points that list at the new ids. The game's banks stay as they are:

```json
{ "type": "timpani_bank", "name": "#e0f28740d6c19769",
  "clips": [
    { "clip": "#1111111111111111", "data": "res/rock.clip", "stream": "res/rock.stream" },
    { "clip": "#1111111111111111", "lang": 256, "data": "res/rock.fr.clip", "stream": "res/rock.fr.stream" },
    { "clip": "#2222222222222222", "data": "res/short.clip" }
  ] }
```

- `clip` is the clip's id, `#` plus 16 hex digits. `lang` is the language flag from the table above, 0 if you leave it out.
- `data` is what goes in the bank: the clip's 68-byte header and the first part of its audio. `stream` is the rest, if any.
- A resource with `clips` has no `bundle`: the clips go into the boot bundle's patch, wherever the bank is. Clips and languages you don't list play as the game has them.
- Two rows can use the same file. Mods can change different clips of one bank, and when two change the same clip, the later one in the load order wins. Over a mod that replaces the whole bank, that bank still deploys, and the languages you don't list play the clip as that mod has it.
- Deploy refuses a clip or language the bank doesn't have, and a file whose header doesn't match its size.

A game update counts as changes waiting, and a new deploy reads the game again. The deployed patch holds the list of sounds (about 0.2 MB) and the new clips, with the game's own clip for each language you leave out where that language plays other audio: one footstep in US English takes about 2 MB of disk.

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
| `clips` | Instead of `data`: the clips of a sound bank to change, each with `clip`, `lang`, `data` and an optional `stream` |
| `bundle` | The bundle the resource goes in, 16 hex digits. Needed for every resource except one with `clips` |
| `flags` | A value bundles keep beside each resource; leave it out unless a converter set it |
| `when` | Options that must have these values for the resource to be used ([Options and keys](wiki:options-and-keys#options-that-pick-files)) |

## Limits

A replaced resource is static: it can't be switched off in the middle of a game. In a session mod, use resources for data the game reads at startup and hooks for behavior.
