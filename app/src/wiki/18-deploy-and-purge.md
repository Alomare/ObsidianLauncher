---
title: Deploy and purge
group: Getting started
summary: What a deploy does, what a purge removes, and why Obsidian sometimes refuses.
---

## Deploy

Deploy puts the enabled mods of the active profile into the game, with the game closed. The button turns green while changes wait that aren't in the game yet, an option value or a key included.

Every deploy replaces all the patches of the one before, so there's no need to purge first: a mod turned off or removed leaves the game at the next deploy. When only option values or keys changed, nothing is rebuilt and the deploy takes a moment. Each deploy runs these steps:

1. Check whether the game is the build Obsidian knows (see below).
2. Back up the save folder when it changed since the last backup. The first backup and the 9 newest are kept, so the save from before any mod stays.
3. Work out which mod wins each resource, following the load order.
4. Compile the Lua of every mod, and the loader, into code the game can run.
5. Write one patch per game bundle, and the list of deployed mods for the loader.

**Play** starts the game with the mods of the last deploy. It doesn't deploy, so waiting changes stay out of the game until **Deploy**. With nothing deployed, Play asks first, since the game would start vanilla. **Don't ask again** stops the question, and **Ask before playing vanilla** in Settings, System brings it back.

## Purge

Purge deletes every file Obsidian wrote and checks that none is left, so the game is the vanilla game again. The library and the profiles stay, and the next deploy brings the mods back.

> [!NOTE]
> Patches that another tool or a person put in the game folder are listed under Game in [Settings](app:settings), and Purge never deletes them. The patches Obsidian writes go above them. **Remove other patches** there moves them out of the game into the `removed-patches` folder in the data folder, where they can be copied back by hand.

> [!WARNING]
> Purge before uninstalling Obsidian. The patches stay in the game folder until something removes them, so the game would keep running the mods.

## The build check

Obsidian was made for one Steam build of HELLDIVERS. When some game files differ from that build, the bar at the top says **Game files changed**, and a click on it shows which files differ and what to do. Deploy and Play still work, and Obsidian never writes over an original game file.

HD1 hasn't had an update since 2020, so this almost always means a mod was copied straight into the `contents` folder. Mods from Obsidian load on top of it, and where both change the same thing, the Obsidian one wins. To get the known files back, purge, verify the game files in Steam, then add the copied mod with **Add**. Adding mods waits until the files match again, since it compares each mod with the original files.

The loader checks the game version every time the game starts. On a version it doesn't know, no mod runs and the game says so.

## Saves

Obsidian never edits progress. While session content is in play it only clears the results waiting to upload, so they can't reach the Galactic War later. Session mods may change progress such as XP and unlocks, and Obsidian doesn't undo that, which is why each deploy makes a backup first. The backups are in the `backups` folder in the app data folder.
