---
title: Replacing game files
group: Making mods
summary: Changing game scripts and other resources by name, without shipping whole bundles.
---

Besides running a script, a mod can replace resources of the game, or add new ones, through the `resources` list. Obsidian puts them into the patches at deploy and works out conflicts with other mods resource by resource.

## A changed game script

Copy the game script to change into the mod folder, edit it, and list it:

```json
"resources": [
  { "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" }
]
```

- `type` and `name` are the game path of the resource.
- `source` is plain Lua in the mod folder. Obsidian compiles it at deploy.
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

Sound banks (`timpani_bank`) and videos (`bik`) keep most of the bytes in the `.stream` file of a bundle. Give that part as `stream`, beside `data`:

```json
{ "type": "timpani_bank", "name": "#e709d0c5697fb286", "bundle": "0e2ec9da60a0f53a",
  "data": "res/menu.timpani_bank", "stream": "res/menu.stream" }
```

A resource the game has in several languages lists each one under `variants`, with the language flag of the game in `lang`. A variant without `lang` is 0, the one used by every language that has no variant:

```json
{ "type": "timpani_bank", "name": "#f0b3f997d587fb77", "bundle": "f0b3f997d587fb77",
  "variants": [
    { "data": "res/voices.timpani_bank", "stream": "res/voices.stream" },
    { "lang": 256, "data": "res/voices.fr.timpani_bank", "stream": "res/voices.fr.stream" }
  ] }
```

| Language | `lang` |
|---|---|
| Every language with no variant | 0 |
| Japanese | 1 |
| French | 256 |
| Spanish (Latin America) | 512 |
| Spanish (Spain) | 1024 |
| Portuguese (Brazil) | 2048 |
| Russian | 4096 |
| Italian | 8192 |
| German | 16384 |

At deploy, Obsidian writes the streamed parts into a `.stream` file beside the patch (`<bundle>.patch_0.stream`), in the order the patch lists them, as the game lays out the original ones. A sound change stays on one PC, so a mod of sound banks can be a [client mod](wiki:scopes).

## Single clips of a sound bank {#sound-bank-clips}

A whole bank can pass a gigabyte across all languages. A mod that changes a few sounds can ship only those clips under `clips`. At deploy, Obsidian points the list of sounds in the game (`timpani_master`, in the boot bundle) at new ids for those clips, and puts the clips under the new ids into a small Obsidian bank beside the bank they replace. The banks of the game stay as they are:

```json
{ "type": "timpani_bank", "name": "#e0f28740d6c19769",
  "clips": [
    { "clip": "#1111111111111111", "data": "res/rock.clip", "stream": "res/rock.stream" },
    { "clip": "#1111111111111111", "lang": 256, "data": "res/rock.fr.clip", "stream": "res/rock.fr.stream" },
    { "clip": "#2222222222222222", "data": "res/short.clip" }
  ] }
```

- `clip` is the id of the clip, `#` plus 16 hex digits. `lang` is the language flag from the table above, 0 when left out.
- `data` is what goes in the bank: the 68-byte header of the clip and the first part of the audio. `stream` is the rest, if any.
- A resource with `clips` has no `bundle`: that small bank goes into every bundle that holds the bank of the game and loads with it, in the language the game is set to. Clips and languages left out play as the game has them.
- Two rows can use the same file. Mods can change different clips of one bank, and when two change the same clip, the later one in the load order wins. Over a mod that replaces the whole bank, that bank still deploys, and the languages you don't list play the clip as that mod has it.
- Deploy refuses a clip or language the bank doesn't have, and a file whose header doesn't match the file size.

A game update counts as changes waiting, and a new deploy reads the game again. The deployed patch holds the list of sounds (about 0.2 MB) and the new clips, with the original clip for each language left out where that language plays other audio: one footstep in US English, which sits in two banks, takes about 4 MB of disk.

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
| `part` | The [part](wiki:manifest#parts) of the mod the resource belongs to |

## Files in parts {#parts}

In a mod with parts, `"part": "<part id>"` puts a resource in that part. A resource of a part that is off isn't deployed, as with `when`. A resource of a client part stays with the player who runs it: it doesn't keep vanilla players out of a public lobby, as the files of a session mod do. A resource without a part follows the strictest part that is on, so in a session or hybrid mod it counts as session content.

## Limits

A replaced resource is static: it can't be switched off in the middle of a game. In a session mod, use resources for data the game reads at startup and hooks for behavior. A part doesn't change that: the files of a session part stay in the game in every lobby, and only the deploy decides which go in.
