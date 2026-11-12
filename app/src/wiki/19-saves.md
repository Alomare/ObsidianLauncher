---
title: Saves
group: Getting started
summary: Official and Sandbox saves, Steam Cloud, and the Sandbox matchmaking pool.
---

The [Saves](app:saves) tab keeps every save of the Steam account signed in on the PC. There are two kinds: **Official** saves are the normal game files, and **Sandbox** saves are extra saves that Obsidian keeps apart from them. The save marked **Active** is the one the game boots next, and nothing here changes while the game runs.

The tab is laid out like the Mods tab: a table of the saves on the left, with rank, samples and time played, and the panel of the selected save on the right. Dragging the line between them resizes the panel, and a double-click on the line resets it. The buttons at the top of the panel are what the selected save can do, each with a tooltip, and the right-click menu of a row has the same actions. A double-click on a Sandbox opens the editor. **New** at the top makes a blank Sandbox or Official save, or imports a Sandbox from a zip.

## Official saves

An Official save is the progress the game and Steam know about. The first time the Saves tab opens, the current game files are added as an Official save named Main. An Official save can:

- **Switch to** an older Official save, or **Restore** a snapshot. Obsidian first keeps the save in use in the library and backs up the game files, then writes the chosen save over them.
- **New** starts a blank Official save, kept in the library until it is switched to.
- **Snapshot** keeps a copy of the save to come back to.
- **Duplicate as Sandbox** copies the progress into a new Sandbox and leaves the Official save as it is.

Official saves can't be renamed, deleted, exported or edited.

### Steam Cloud

The game loads the Steam Cloud copy first, so a switch of Official save, with the game closed, writes the Cloud files as well as the local ones. Steam uploads the new save at the next sync and shows no dialog. When Steam isn't tracking the files, Obsidian writes only the local ones, and the game makes the Cloud files again at the next save. A switch also clears the results waiting for the game servers, so an old save doesn't report old missions.

The backups of the game files are in the backups folder ([Where the files are](wiki:how-obsidian-works#where-your-files-are)).

## Sandbox saves

A Sandbox lives only on this PC, so it never touches Steam Cloud or the Official files. Sandbox saves can be created, duplicated (also from an Official save), renamed, deleted, exported to a zip, imported from one, and [edited](#the-save-editor).

Every new Sandbox, blank, duplicated or imported, starts with all the tutorials done: boot camp, the galactic war tutorials, the introduction and hints of the admiral on the ship, and the hellpod arrows.

A Sandbox needs the Obsidian loader in the game, so the mods have to be deployed. Pressing Play with a Sandbox selected and nothing deployed asks to [Deploy](wiki:deploy-and-purge) first. Without the loader the game opens the Official save, untouched.

### The Sandbox matchmaking pool

Sandbox saves can't move the official war. While a Sandbox is active:

- Reporting to the game servers is blocked and waiting results are dropped.
- Achievements are blocked.
- Sandbox players only see and join other Sandbox lobbies, and the multiplayer terminal says **Sandbox Public** above the lobby list. Friends are included: a player on a Sandbox and a friend on an Official save can't join each other, and the refused invite says so.

Official saves can join anything, as always ([Playing online](wiki:multiplayer)). A custom war from Commander plays on a Sandbox save, in a separate pool: any Sandbox, or the one made for the war when none is picked or the host keeps the war to it.

## The save editor {#the-save-editor}

**Edit** on a Sandbox opens the editor, with the game closed. Changes wait for **Save**, which writes them all at once. **Discard**, the arrow beside Save, forgets them all, and the arrow at the end of a row undoes only the change on that row. The tabs are:

- **Overview**: the stats of the save, **Max everything** and **Reset progression**. Both ask first and write at once, so they need no other changes waiting.
- **Character**: the rank (a dropdown tied to the XP field), samples with the research points they make, cosmetics, boss influence and tutorial flags.
- **Arsenal**: weapons, stratagems and perks, one table per category, with a search box. Each row has an owned switch, weapons have named upgrades, and stratagems have a level.
- **Mods**: one section per mod with save data. A mod that is off greys out and keeps the data. A mod no longer in the library lists the data by name, with a delete button.

Item names and limits come from a list taken from the game. Items in the save that the list doesn't know are kept and shown with their raw names. An item that comes with a rank can't be switched off, since the rank gives it back.

### Paid DLC

Obsidian reads which DLC the Steam account owns from the Official saves, where the game records it at each login.

- Owned DLC shows **Owned on Steam**. Steam grants it, so the owned switch stays on, but the upgrades can change, and owned DLC cosmetics can be picked. **Max everything** maxes the upgrades of owned DLC, and **Reset progression** clears them.
- DLC the account doesn't own shows locked, with a lock in place of the switch, and the cosmetics lists leave those cosmetics out. The editor never writes that DLC, and Max everything and Reset progression leave it alone.

The check is only as fresh as the last login. DLC bought since then, or unticked in Steam at that login, counts as not owned until the game opens once with an Official save. The record inside a Sandbox is ignored, since an imported Sandbox can carry the record of another player.

> [!NOTE]
> Exported and imported saves are always Sandbox saves. Import checks the file, removes the waiting results, and removes paid DLC the account doesn't own from the imported save.
