---
title: Deploy and purge
group: Getting started
summary: What happens when you deploy, what a purge removes, and why Obsidian sometimes refuses.
---

## Deploy

Deploy puts the enabled mods of the active profile into the game. The button turns green when you have changes that are not in the game yet, an option value or a key included. The game has to be closed.

Every deploy replaces all the patches of the one before, so there's no need to purge first: a mod you turned off or removed leaves the game at the next deploy. When only option values or keys changed, nothing is rebuilt and the deploy takes a moment.

Each deploy runs these steps:

1. Check whether the game is the build Obsidian knows (see below).
2. Back up your save folder, when it changed since the last backup. The first backup and the 9 newest are kept, so your save from before any mod stays.
3. Work out which mod wins each resource, following the load order.
4. Compile the Lua of every mod, and the loader, into code the game can run.
5. Write one patch per game bundle, and the list of deployed mods for the loader.

**Play** starts the game with the mods you last deployed. It doesn't deploy, so changes that are waiting stay out of the game until you press **Deploy**. With nothing deployed, Play asks first, as the game would start vanilla. **Don't ask again** stops the question, and **Ask before playing vanilla** in Settings, System brings it back.

## Purge

Purge deletes every file Obsidian wrote and checks that none is left. The game is then the vanilla game again. Your library and profiles stay; deploy again to bring the mods back.

> [!NOTE]
> Patches that another tool or a person put in the game folder are listed under Game in [Settings](app:settings), and Purge never deletes them. The patches Obsidian writes go above them. **Remove other patches** there moves them out of the game, into the `removed-patches` folder in the data folder, where you can copy them back by hand.

> [!WARNING]
> Purge before you uninstall Obsidian. The patches stay in the game folder until something removes them, so the game would keep running your mods.

## The build check

Obsidian was made for one Steam build of HELLDIVERS. If some game files differ from it, the bar at the top says **Game files changed**. Click it to see which files differ and what to do. Deploy and Play still work, and Obsidian never writes over the game's own files.

HD1 hasn't been updated since 2020, so this almost always means a mod was copied straight into the `contents` folder. Obsidian's mods load on top of it, and where both change the same thing, Obsidian's wins. To get the known files back, purge, verify the game files in Steam, then add the copied mod with **Add**. Adding mods waits until the files match again, since it compares each mod with the original files.

The loader checks the game's version every time the game starts. On a version it doesn't know, no mod runs and the game says so.

## Saves

Obsidian never edits your progress. While session content is in play it only clears the results waiting to upload, so they can't reach the Galactic War later. Session mods may change your own progress (XP, unlocks), and Obsidian doesn't undo that, which is why each deploy makes a backup first. The backups are in the `backups` folder in the app data folder.
