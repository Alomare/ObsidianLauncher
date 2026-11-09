---
title: How Obsidian works
group: Getting started
summary: What Obsidian does to the game, what stays untouched, and what each tab is for.
---

Obsidian is a launcher for HELLDIVERS (2015) that manages your mods. You keep your mods in a library, choose which ones to play with, and Obsidian puts them into the game. Your game files are never edited, so several mods can run side by side and the game can always go back to vanilla.

## Patches instead of replaced files

The game keeps the content in bundles, in the `contents` folder of the install. Next to each bundle, the engine also loads files named `<bundle>.patch_0`, `.patch_1` and so on, and a resource in a patch takes the place of the one in the bundle.

Mods made by hand usually replace whole bundles. Two such mods overwrite each other, even when they change different things. Obsidian reads every enabled mod, finds the resources each one really changes, and writes them into one patch per bundle. The vanilla bundles stay as they were.

> [!NOTE]
> Deploy writes the patches, and [Purge](wiki:deploy-and-purge#purge) deletes them. After a purge the install is byte for byte the vanilla game.

## Obsidian Loader

Mods that run code, such as a camera mod, need a way to start inside the game. Obsidian adds one small script to the game at every deploy: Obsidian Loader. When the game boots, the loader reads the list of deployed mods, starts each one, and keeps an eye on what they are allowed to do.

The loader also handles the multiplayer rules. In a public lobby, mods that change the shared game hold back until every player allows them, and modded missions are kept out of the Galactic War. [Scopes](wiki:scopes) explains how.

## The tabs

| Tab | What it is for |
|---|---|
| Home | Links to the community and the source code |
| [Mods](app:mods) | Your library: turn mods on and off, set the load order, change options and keys |
| Wiki | These guides |
| [Settings](app:settings) | Language, theme, game folder, updates |

Home and Mods sit on the left of the bar. The Wiki (the book) and Settings (the cog) are the two icons at the right end.

The bar at the top holds [Guardrails](wiki:guardrails), the shield, and three buttons:

- **Purge** takes every Obsidian patch out of the game.
- **Deploy** puts the enabled mods into the game. It turns green when your changes have not been deployed yet.
- **Play** starts the game through Steam.

The status bar at the bottom shows what Obsidian is doing, the version, your notifications and the About box.

## Updating Obsidian

The **Updates** tab in [Settings](app:settings) shows your version, checks for a new one and lists what each version changed. **When a new version is out**, in the System tab, decides what happens at startup: **Show it here**, **Ask at startup** or **Install it**. The list opens the installed version and any newer ones, and you can open older ones.

While a new version downloads, a window in front of the app shows the progress. Obsidian then closes, the installer runs for a moment, and Obsidian opens again on the new version.

A portable copy doesn't install new versions. Download them from the releases page and unzip them over the old copy.

## Where your files are

| What | Where |
|---|---|
| Mod library, profiles, app settings | `%LOCALAPPDATA%\ObsidianLauncher`, or `data\` beside the app in a portable copy |
| Options and keys the game reads, logs | `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher` |
| Save backups | the `backups` folder in the app data folder |
