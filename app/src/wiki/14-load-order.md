---
title: Load order, conflicts and folders
group: Getting started
summary: Which mod wins when two change the same thing, and how to keep a long list tidy.
---

## Load order

The number beside each mod is the place of that mod in the load order. When two enabled mods change the same game resource, the one lower in the list wins. Drag a mod up or down to change the order, or select it and use Alt+Up and Alt+Down.

## Sort Mods

A mod can ask for a place in the load order: first, last, before or after other mods, or before or after every mod that replaces game files. The **Sort Mods** button, right after the profile at the top of the Mods tab, puts the list in line with those requests, for mods that are on and off alike.

Nothing moves before **Apply**. The Sort Mods window first lists the whole new order, with each moved mod marked, the old and new place, and the reason. Hovering over a reason that mentions files shows them. After Apply, **Undo** in the bar at the foot of the list puts the old order back.

- Mods that ask for nothing keep their order, and as few mods move as possible.
- When two orders move as many mods, the one where mods replacing the same files lose less wins. The later mod takes a shared file whole, so the mod with fewer files goes later: it overwrites a smaller part of the other mod and isn't lost entirely.
- Requests that can't all hold, such as two mods that both ask to load last, aren't guessed at. Those mods stay in place, and the window names them under **Not moved** for ordering by hand.
- Mods in a locked folder don't move, and no mod goes inside a locked folder. A moved mod that lands inside another folder joins it, and one that leaves a folder goes out of it.

A mod also takes the place it asks for when it joins a profile. After that the order belongs to the player: a moved mod stays moved until the next sort.

## Conflicts

A warning sign on a row means the mod shares resources with another enabled mod. Hover over it to see which mods and how many files, or click it to open the list in the panel on the right. A yellow sign means this mod overrides files of another mod, and a grey sign means another mod overrides some files of this one.

A conflict is often fine. Two mods may change the same script on purpose, and the order decides which version reaches the game. When the result looks wrong in game, try the other order first.

> [!TIP]
> Conflicts only count the resources a mod would deploy with the options chosen. An option that is off never causes a conflict.

The lists that tell the game which units exist are the exception. Mods that add a new weapon or unit each change those lists, so Obsidian merges them, and those mods work together in any order. Everyone in a game still needs the same mods.

## Folders

Folders group mods in the list. They change nothing in the game: the load order still runs from the top of the list to the bottom, through every folder.

- The **Create Folder** button at the top of the Mods tab makes one, and right-clicking checked mods puts them in a new folder.
- Drag mods onto a folder name to move them in, and drag the folder to move the mods in it together.
- Click the arrow to fold a folder away.
- Right-click a folder to rename it, change the color or delete it. Deleting asks whether the mods go too or stay in the list.

### Locked folders

The padlock before the name of a folder locks it. A locked folder keeps the name, color, place and mods it has, and nothing moves in or out until it is unlocked.

## Checking several mods

The checkboxes on the left select mods for batch actions, and Shift-click checks a range. With mods checked, the bar at the foot of the list turns them on or off, checks them for updates or removes them. Right-clicking one of them offers everything else, such as moving them all to a folder.
