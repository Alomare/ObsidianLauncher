---
title: How Obsidian works
group: Getting started
summary: What Obsidian does to the game, what stays untouched, and what each tab is for.
---

Obsidian is a launcher and mod manager for HELLDIVERS (2015). Mods live in a library, a profile picks which ones play, and Obsidian puts them into the game. No game file is ever edited, so several mods can run side by side and the game can always go back to vanilla.

## Patches instead of replaced files

The game keeps the content in bundles, in the `contents` folder of the install. Next to each bundle, the engine also loads files named `<bundle>.patch_0`, `.patch_1` and so on, and a resource in a patch takes the place of the one in the bundle.

Mods made by hand usually replace whole bundles, so two such mods overwrite each other even when they change different things. Obsidian reads every enabled mod, finds the resources each one really changes, and writes them into one patch per bundle. The vanilla bundles stay as they were.

> [!NOTE]
> Deploy writes the patches, and [Purge](wiki:deploy-and-purge#purge) deletes them. After a purge the install is byte for byte the vanilla game.

## Obsidian Loader

Mods that run code, such as a camera mod, need a way to start inside the game. At every deploy Obsidian adds one small script to the game, Obsidian Loader. When the game boots, the loader reads the list of deployed mods, starts each one, and watches what they are allowed to do.

The loader also applies the multiplayer rules. In a public lobby, mods that change the shared game hold back until every player allows them, and modded missions stay out of the Galactic War. [Scopes](wiki:scopes) explains how.

## The tabs

| Tab | What it is for |
|---|---|
| Home | Buttons for the Store tab, Nexus Mods, the Discord server, Ko-fi and the source code on GitHub |
| [Store](app:store) | Mods to install in one click ([more](wiki:tab-store)) |
| [Mods](app:mods) | The library: mods on and off, load order, options, keys, **Deploy** and **Purge** ([more](wiki:tab-mods)) |
| [Saves](app:saves) | Official and Sandbox saves, and the save editor ([more](wiki:tab-saves)) |
| [Wars](app:commander) | Custom Galactic Wars, covered in the Commander subtab of this Wiki ([more](wiki:tab-wars)) |
| Wiki | These guides ([more](wiki:tab-wiki)) |
| [Account](app:account) | The GitHub sign-in, issue reports and sync between PCs ([more](wiki:tab-account)) |
| [Settings](app:settings) | Game folder, language, updates, controller and appearance ([more](wiki:tab-settings)) |

The bar at the top of the window holds, from left to right:

- The tabs Home, Store, Mods, Saves and Wars.
- The [Guardrails](wiki:guardrails) button, a shield.
- **Play**, which starts the game through Steam with the mods of the last deploy. When there is more than one war or save to choose from, a menu button beside **Play** picks the war and the save that **Play** starts.
- Three icons: the Wiki (a book), the Account (a person) and Settings (a cog). A dot on the cog means a new version of Obsidian is out.

**Deploy** and **Purge** are at the right end of the [Mods](app:mods) tab ([Deploy and purge](wiki:deploy-and-purge)).

The status bar at the bottom shows what Obsidian is doing, the version, the notifications and the About box.

## Updating Obsidian

The **Updates** tab in [Settings](app:settings) shows the installed version, checks for a new one and lists what each version changed. **When a new version is out**, in the System tab, decides what happens at startup: **Show it here**, **Ask at startup** or **Install it**. The list opens with the installed version and any newer ones, and older ones open on a click.

While a new version downloads, a window in front of the app shows the progress. Obsidian then closes, the installer runs for a moment, and Obsidian opens again on the new version. A portable copy doesn't install new versions: download them from the releases page and unzip them over the old copy.

## Where the files are {#where-your-files-are}

| What | Windows | Linux |
|---|---|---|
| Mod library, profiles, app settings | `%LOCALAPPDATA%\ObsidianLauncher`, or the `data` folder beside the app in a portable copy | `~/.local/share/ObsidianLauncher` |
| Options and keys the game reads, logs | `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher` | The same folder inside the Proton prefix of the game: `drive_c/users/steamuser/AppData/Roaming/Arrowhead/Helldivers/ObsidianLauncher` |
| Save backups | The `backups` folder in the app data folder | The same |

**Data folder** in the System tab of [Settings](app:settings) opens the app data folder, and **Open logs** opens the logs.
