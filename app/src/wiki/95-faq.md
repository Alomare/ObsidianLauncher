---
title: F.A.Q
group: FAQ
summary: Short answers, each with a link to the page that covers the subject in full.
---

## Playing

### Will a mod get me banned?

HELLDIVERS has no anti-cheat, and Obsidian never writes over a game file. Session mods are kept out of the Galactic War, and in a public lobby they switch off while a player there doesn't allow them ([Scopes](wiki:scopes)).

### Can I play with friends who don't use Obsidian?

Yes. Session mods don't change who you find or who can join you. In a friends-only or private lobby they run with anyone, and in a public lobby they switch off while a vanilla player is there. Data overrides and older imported mods can't switch off, so while you host a public lobby with them, vanilla players can't join it. The first time you load into the ship, a message says which case applies to you ([Playing online](wiki:multiplayer#matchmaking)).

### My friend and I both use Obsidian. Why are my session mods off?

In a public lobby, session mods run when every other player has the same set or doesn't block other players' session mods. Set the lobby to friends only and they run with anyone. Exporting your profile and sending it to your friend is the quickest way to match ([Profiles](wiki:profiles#sharing-a-profile)).

### Do my missions count for the Galactic War?

Not while session content is deployed, and not when the content of an expansion runs in them. With only client mods and expansions they count as normal ([The Galactic War](wiki:galactic-war)).

### Is my save safe?

Before a deploy, Obsidian backs up the save folder when it changed since the last backup. The first backup and the 9 newest are kept, so the save from before any mod stays ([Deploy and purge](wiki:deploy-and-purge#saves)).

### The bar says Game files changed. What now?

Some game files differ from the build Obsidian knows, usually because a mod was copied straight into the game's `contents` folder. Deploy and Play still work. Click the warning to see which files differ and how to get the originals back ([Troubleshooting](wiki:troubleshooting#build-unknown)).

## Mods

### Can I use mods made before Obsidian?

Yes. Drop the mod onto the list as usual. Obsidian keeps only the parts that differ from the game, so older mods can run together ([Adding and updating mods](wiki:adding-mods#older-mods)).

### Two mods change the same thing. Which one wins?

The one lower in the list ([Load order](wiki:load-order)).

### Do I need an ultrawide mod?

No. Obsidian Patches removes the black bars on wide screens, and that option starts on. An imported ultrawide patch does the same job.

### What is Obsidian Patches?

A client mod that comes with Obsidian, made of small fixes that are each an option. Starting on: no intro, login screen or planet scanning, no black bars on wide screens, faster results screens after a mission, a pause that freezes a solo game, and two animation fixes, the weapon recoil after a stratagem throw and the sprint pose after a dive. Ready to switch on: skipping war briefs, an aim cursor that stays visible, and a minimap that stays open. It sits in the Obsidian Kit folder at the top of the list.

## Making mods

### Do I need any tools?

A text editor. Obsidian compiles Lua when it deploys ([Your first mod](wiki:first-mod)).

### Client or session?

If other players could notice the change, session. If every change happens inside content that stands apart from the normal game, such as a new mode, expansion. When in doubt, session ([Scopes](wiki:scopes)).
