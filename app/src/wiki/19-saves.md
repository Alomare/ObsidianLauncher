---
title: Saves
group: Getting started
summary: Official and Sandbox saves, Steam Cloud, and the Sandbox matchmaking pool.
---

The [Saves](app:saves) tab keeps every save of the Steam account you are signed in with. There are two kinds: **Official** saves are the normal game files, and **Sandbox** saves are extra saves that Obsidian keeps apart from them. The save marked **Active** is the one the game boots next. The game has to be closed to change anything here.

The tab is laid out like the Mods tab: a table of the saves on the left, with rank, samples and time played, and the panel of the selected save on the right. Drag the line between them to resize the panel, and double-click the line to reset it. The buttons at the top of the panel are what the selected save can do; hover one to see what it does. The same actions are in the right-click menu of a row, and a double-click on a Sandbox opens the editor. **New** at the top makes a blank Sandbox or Official save, or imports a Sandbox from a zip.

## Official saves

An Official save is the progress the game and Steam know about. The first time you open the Saves tab, your current game files are added as an Official save named Main.

What an Official save can do:

- **Switch to** an older Official save, or **Restore** a snapshot. Obsidian first keeps the save in use in the library and backs up the game files, then writes the chosen save over them.
- **New** starts a blank Official save, kept in the library until you switch to it.
- **Snapshot** keeps a copy of the save to come back to.
- **Duplicate as Sandbox** copies the progress into a new Sandbox and leaves the Official save as it is.

Official saves can't be renamed, deleted, exported or edited.

### Steam Cloud

The game loads the Steam Cloud copy first, and Obsidian writes the Cloud files as well as the local ones when you switch an Official save with the game closed. Steam then uploads the new save the next time Steam syncs, and shows no dialog. If Steam isn't tracking the files, Obsidian writes only the local ones, and the game makes the Cloud files again on the next save. A switch also clears the results waiting to be sent to the game's servers, so an old save doesn't report old missions.

Backups of your game files are listed in [Settings](wiki:how-obsidian-works#where-your-files-are).

## Sandbox saves

A Sandbox lives only on this PC, so it never touches Steam Cloud or your Official files. Sandbox saves can be created, duplicated (also from an Official save), renamed, deleted, exported to a zip, imported from one, and [edited](#the-save-editor).

A Sandbox needs the Obsidian loader in the game, so the mods have to be deployed. If you press Play with a Sandbox selected and nothing deployed, Obsidian asks to [Deploy](wiki:deploy-and-purge) first. Without the loader the game opens your Official save, untouched.

### The Sandbox matchmaking pool

Sandbox saves can't progress the real war. While a Sandbox is active:

- Reporting to the game's servers is blocked and waiting results are dropped.
- Achievements are blocked.
- Sandbox players only see and join other Sandbox lobbies, and the multiplayer terminal says **Sandbox Public** above the lobby list. Friends are included: a friend on an Official save can't join you, and you can't join them. The refused invite says so.

Official saves can join anything, as always. See [Playing online](wiki:multiplayer).

## The save editor {#the-save-editor}

**Edit** on a Sandbox opens the editor, with the game closed. Changes wait until you press **Save**, which writes them all at once. **Discard**, the arrow beside Save, forgets them all, and the arrow at the end of a row undoes only the change on that row. The tabs are:

- **Overview**: the stats of the save, **Max everything** and **Reset progression**. Both ask first, and both write at once, so they need no other changes waiting.
- **Character**: the rank (a dropdown tied to the XP field), samples with the research points they make, cosmetics, boss influence and tutorial flags.
- **Arsenal**: weapons, stratagems and perks, one table per category, with a search box. Each row has an owned switch, weapons have named upgrades, and stratagems have a level.
- **Mods**: one section per mod that has save data. A mod that is off greys out and keeps the data. A mod no longer in the library lists the data by name, with a delete button.

Item names and limits come from a list taken from the game. Items in the save that the list doesn't know are kept and shown with their raw names. An item a rank gives you can't be switched off, since the rank gives it back.

### Paid DLC

Obsidian reads which DLC your Steam account owns from your Official saves: the game records it there each time you log in.

- DLC you own shows **Owned on Steam**. Steam grants it, so the owned switch stays on, but the upgrades can be changed, and owned DLC cosmetics can be picked. **Max everything** maxes the upgrades of owned DLC, and **Reset progression** clears them.
- DLC you don't own shows locked, with a lock in place of the switch, and the cosmetics lists leave those cosmetics out. The editor never writes that DLC, and Max everything and Reset progression leave it alone.

The check is only as fresh as your last login. DLC bought since then, or unticked in Steam at that login, counts as not owned until the game is opened once with an Official save. A Sandbox's own record is ignored, since an imported Sandbox can carry another player's.

> [!NOTE]
> Exported and imported saves are always Sandbox saves. Import checks the file, removes the waiting results, and removes paid DLC that your account doesn't own from the imported save.
