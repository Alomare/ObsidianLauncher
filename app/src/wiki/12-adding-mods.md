---
title: Adding and updating mods
group: Getting started
summary: Importing a mod from a .zip or a folder, updating one, and removing one.
---

## Importing

Drag a mod folder or a `.zip` file onto the mod list in the [Mods](app:mods) tab, or click the green **Add** button above the list and choose **Import .zip** or **Import folder**. Several files can go in at once, and a notification says what happened to each. The [Store](app:store) installs mods without a download by hand.

Obsidian copies the mod into the library, so the download can be deleted afterwards. Importing changes nothing in the game until the next deploy.

### Mods made for Obsidian

A mod made for Obsidian has an `obsidian.json` file at the top of the folder or archive. Obsidian reads the name, version, options and keys from that file and shows them in the panel on the right.

### Older mods

Most HD1 mods shared today replace whole game bundles, with files named like `9e13b2414b41b842`. Obsidian imports those as well. It compares each bundle in the mod with the one in the game and keeps only the resources the mod really changes, so two older mods that touch different things can run together.

Some older mods bring `.patch_0` or `.patch_1` files instead, named like `9e13b2414b41b842.patch_0`, which the game reads over the matching bundle. Obsidian imports those too, with no renaming to the next free number by hand. A mod that is only a ReShade preset (a `ReShadePreset.ini`) isn't a game mod: it needs ReShade in the game folder, and the import says so.

Older mods come in with the **unspecified** scope, because Obsidian can't tell what they change in a shared game. They run as client mods, so they never keep a player out of a lobby, and they can't switch off in the middle of a mission ([Scopes](wiki:scopes)). An older mod that changes only sound banks comes in as a **client** mod, since a sound change stays on one PC.

A patch for another mod doesn't bring that mod with it. Import both and put the patch lower in the list. Check which version the patch was built for: the files the two share come from the patch, so a patch for an older version takes them back to that version.

> [!NOTE]
> Comparing bundles needs the original game files, so the import stops when they differ. A mod copied into the game folder by hand causes this: verify the game files in Steam and import again.

## Updating

Import the new version the same way. A mod with the same id replaces the one in the library, and the notification shows the old and new version numbers. Option values and keys stay, except for options whose type the author changed.

### From GitHub

A mod can name a GitHub repository. Obsidian asks GitHub for new versions of those mods at startup, unless it asked in the last six hours, and on request:

- **Check All for Updates**, an icon at the top of the [Mods](app:mods) tab, asks about every such mod.
- **Check for Updates** in the bar at the foot of the list asks about the checked mods. The checkbox of a folder checks every mod in it, and the one at the top of the list checks them all.

A mod with a new version shows it in gold in the Version column, and **Update to** in the details of the mod. A click on either shows the new version and a link to the changelog, then asks to confirm. The gold button with a number, at the top of the Mods tab, installs every waiting update at once. Only stable releases with a higher version are offered, and nothing installs without a click. Deploy afterwards to put the new versions in the game.

The Versions tab in the details of such a mod lists every release, by channel, with what each one changed, and installs any of them, older ones included ([Sharing a mod](wiki:sharing#updates) explains channels).

## Removing

Right-click a mod and choose **Remove**, or use the bin under the close button in the details of a mod. To remove several, check them and use **Remove** in the bar at the foot of the list.

## Obsidian Kit

Three mods come with Obsidian, in the **Obsidian Kit** folder at the top of the list: Obsidian API, [Obsidian Trainer](wiki:trainer) and Obsidian Patches. Obsidian Patches starts on, with most of the fixes enabled, and the other two stay off until switched on.

They are mods like any other: they turn on and off, can be removed and update from GitHub. The folder starts locked, so it stays in place. A click on the padlock unlocks it, and then the mods can move and the folder can be renamed or deleted. A removed Kit mod installs again from the Obsidian Kit row at the top of the [Store](app:store).

## Dependencies

Some mods need others to work. The **Dependencies** tab, in the panel on the right, shows what a mod needs and which mods need it. Switching a mod on switches on what it needs, and switching it off switches off the mods that need it. When a mod needs something missing from the library, the tab says so, and the mod stays off until that mod is added.

## Pictures

A mod can bring a square picture. To set another one, open the mod in the panel on the right and click the picture. Profiles take a picture the same way.
