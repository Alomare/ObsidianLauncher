---
title: Adding and updating mods
group: Getting started
summary: Importing a mod from a .zip or a folder, updating one, and removing one.
---

## Importing

Drag a mod folder or a `.zip` file onto the mod list in the [Mods](app:mods) tab, or click the green **+** beside Create Folder and pick one. Several files can go in at once, and a notification says what happened to each one.

Obsidian copies the mod into the library, so you can delete the download afterwards. Importing changes nothing in the game until you deploy.

### Mods made for Obsidian

A mod made for Obsidian has an `obsidian.json` file at the top of the folder or archive. Obsidian reads the name, version, options and keys from that file, and the mod shows them in the panel on the right.

### Older mods

Most HD1 mods shared today replace whole game bundles, with files named like `9e13b2414b41b842`. Obsidian imports those as well. It compares each bundle in the mod with the one in your game and keeps only the resources the mod really changes, so two older mods that touch different things can now run together.

Some older mods bring `.patch_0`, `.patch_1` files instead, named like `9e13b2414b41b842.patch_0`, which the game reads over its own bundle. Obsidian imports those too, so there's no renaming them to the next free number by hand. A mod that is only a ReShade preset (a `ReShadePreset.ini`) isn't a game mod: it needs ReShade in the game folder, and the import says so.

Older mods are imported with the **unspecified** scope, because Obsidian can't tell what they change in a shared game. They run as client mods, so they never keep you out of a lobby ([Scopes](wiki:scopes)). An older mod that changes only sound banks comes in as a **client** mod, since sound changes only what you hear. They also can't be switched off in the middle of a mission.

A patch for another mod doesn't bring that mod with it. Import both and put the patch lower in the list. Check which version the patch was built for: the files the two share come from the patch, so a patch for an older version takes them back to that version.

> [!NOTE]
> To compare bundles, the game files have to be the original ones, so the import stops if they aren't. If you copied a mod into the game folder by hand, verify the game files in Steam and import it again.

## Updating

Import the new version the same way. A mod with the same id replaces the one in the library, and the notification shows the old and new version numbers. Your option values and keys stay, except for options the author changed the type of.

### From GitHub

A mod can name its GitHub repository. When one does, a button with two arrows appears on its row in the [Mods](app:mods) tab, between the switch and the name, and asks GitHub for a new version of that mod. The same button on a folder asks about every mod in that folder, and the one at the top of the list asks about them all. A mod that has a new version shows an **Update to** button beside its name. Click that, then deploy.

Obsidian asks GitHub when it starts and when you click, and it only offers a higher version of the same mod. Nothing installs until you choose to. A mod with a new version shows its version number in a yellow box with an arrow. Click it to see the new version and a link to the mod's changelog, then confirm. **Update all** installs every waiting update at once.

The About tab in the details of such a mod lists the changes in each release, under **Changelogs**. Click a version to read what changed; **Load more** brings older ones.

## Verified mods

A round mark with a check after the name of a mod means the mod is trusted and endorsed by the Obsidian team. The mark shows only when the mod comes from the GitHub repository the team endorsed, so a copy under the same id from elsewhere goes without it. The bundled mods and Better HUD have it. The mark changes nothing else: verified mods deploy and run like any other.

## Removing

Right-click a mod and choose **Remove**, or use the bin under the close button in the details of a mod. To remove several, check them and use **Remove** in the bar at the foot of the list.

## Obsidian Kit

Three mods come with Obsidian, in the **Obsidian Kit** folder at the top of the list: Obsidian API, [Obsidian Trainer](wiki:trainer) and Obsidian Patches. Obsidian Patches starts on, with most of the fixes enabled. The other two stay off until you turn them on.

They're mods like any other: you can turn them on or off, remove them and update them from GitHub. The folder starts locked, so it stays where it is. Click its padlock to unlock it, and then you can move the mods, or rename or delete the folder. To put everything back, use **Restore Obsidian Kit** in [Settings](app:settings). It brings back deleted mods and the folder, and resets their settings in the active profile.

## Dependencies

Some mods need others to work. The **Dependencies** tab, in the panel on the right, shows what a mod needs and which mods need it. Switching a mod on switches on what it needs, and switching it off switches off the mods that need it.

If a mod needs something that isn't in your library, the tab says so, and the mod won't switch on until you add it.

## Pictures

A mod can bring a square picture. To use your own, open the mod in the panel on the right and click the picture. Profiles take a picture the same way.
