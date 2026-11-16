---
title: Mods
group: Know your tabs
summary: The library of mods, the load order, and the details of each mod.
---

The [Mods](app:mods) tab is the library: every mod Obsidian knows, in load order, and which ones the active profile has on.

## The top row

| Control | What it does |
|---|---|
| Profile button | Shows the active [profile](wiki:profiles), and switches to another one or opens **Manage Profiles** |
| **Sort Mods** icon | Orders the list by what the mods ask for ([Sort Mods](wiki:load-order#sort-mods)) |
| **Check All for Updates** icon | Asks GitHub for new versions of every mod that names a repository ([Updating](wiki:adding-mods#updating)) |
| **Create Folder** icon | Makes a folder in the list ([Folders](wiki:load-order#folders)) |
| **Add** | Imports a `.zip` or a folder, or opens the [Store](app:store) |
| Gold button with a number | Shows when updates were found, and installs all of them |
| Search box | Keeps only the mods whose name, description, id or author contains the words typed |
| **Purge** icon | Takes every Obsidian file out of the game ([Purge](wiki:deploy-and-purge#purge)) |
| **Deploy** icon | Puts the enabled mods into the game, and turns green while changes wait ([Deploy](wiki:deploy-and-purge#deploy)) |

## The list

Each row has a check box, the place in the load order, a switch, the mod, the size, what the last deploy put in the game, the version and the [scope](wiki:scopes). A right click on the header shows or hides the **#**, **Size**, **Deployed** and **Version** columns.

- The switch turns a mod on or off in the active [profile](wiki:profiles). Nothing reaches the game until **Deploy**.
- Rows drag to a new place in the [load order](wiki:load-order), and folders group them.
- A warning icon beside a name means the mod changes the same files as another mod; a click shows which.
- A gold version means a newer one was found; a click installs it.
- Checked mods get a bar at the bottom to turn them on or off, check them for updates or remove them. The switch of a checked mod turns every checked mod on or off.

Keys on the list, outside the search box and with no dialog or menu open:

| Key | What it does |
|---|---|
| Ctrl+A | Checks every mod the list shows, with the search applied; again, unchecks them |
| Ctrl+C | Copies the checked mods, or the open one, as a list of names and versions |
| Ctrl+V | Checks the mods a pasted list names, by name or id; pasted file paths import those mods |
| Ctrl+F | Goes to the search box |
| Alt+Up, Alt+Down | Move the focused mod or folder |

A right click on a row or a folder opens a menu with everything else.

Mods can also be dropped on the list. [Adding mods](wiki:adding-mods) covers the formats Obsidian reads.

## The details of a mod

A click on a mod opens the details at the right:

| Tab | What it shows |
|---|---|
| About | The description, the authors and the links |
| Versions | The releases on GitHub, by channel, and a switch to any of them |
| Options | The settings of the mod, saved in the profile. Some mods also show settings in the game, under **Mods** in the pause menu, and a value changed there comes back to this tab once the game is closed |
| Keys | The keys and controller inputs of the mod. **Change binds** opens a table to set a new key or mouse button, a controller input (one to four buttons held together) and an input mode for each |
| Files | The files of the mod, and the conflicts with other mods |
| Dependencies | The mods this one needs, and the mods that need it |
