---
title: Your first mod
group: Making mods
summary: A working mod from two files, with no compiler or other tools to install.
---

An Obsidian mod is a folder with an `obsidian.json` file that describes the mod, plus the files it uses. A script mod needs one Lua file. Obsidian compiles the Lua when it deploys, so you write plain text and never touch the game's own tools.

## 1. Make the folder

Make a folder anywhere, for example `hello-helldiver`, with two files in it:

```
hello-helldiver/
  obsidian.json
  main.lua
```

## 2. Describe the mod

`obsidian.json`:

```json
{
  "obsidian": 1,
  "id": "yourname.hello-helldiver",
  "version": "1.0.0",
  "name": { "en": "Hello Helldiver" },
  "description": { "en": "Writes a line to the log when the game starts." },
  "authors": ["Your name"],
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1
}
```

- `id` is permanent and must be unique. Start it with your name, so two authors never pick the same one.
- `scope` is `client` because this mod changes nothing other players could notice ([Scopes](wiki:scopes)).
- `entry` is the script the loader starts.

## 3. Write the script

`main.lua`:

```lua
obsidian.log("Hello from my first mod")

obsidian.after_startup(function()
  obsidian.log("Every mod has started")
end)
```

The loader gives every mod a global table named `obsidian` with the functions of the [Loader API](wiki:loader-api).

## 4. Try it

1. Drag the folder onto the mod list in the [Mods](app:mods) tab.
2. Turn the mod on and click **Deploy**.
3. Start the game, then close it.
4. Open `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager\logs`. `yournamehello-helldiver.log` has your two lines, and `loader.log` shows the mod starting. A mod log is named after the mod id, keeping only letters, digits, `_` and `-`.

To change the mod, edit the files and import the folder again: a mod with the same id replaces the old copy. Raise `version` when you share a new release.

> [!TIP]
> The mods that come with Obsidian are normal mods too. Obsidian Patches is a good example of a client mod with options; the files are in [its repository](https://github.com/Alomare/ObsidianPatches).

## Next steps

- [The obsidian.json file](wiki:manifest) lists every field.
- [Options and keys](wiki:options-and-keys) lets players set up your mod in the app.
- [Hooks](wiki:loader-api#hooks) are how a mod changes what the game does.
- [Writing session mods](wiki:session-mods) matters as soon as your mod changes the shared game.
