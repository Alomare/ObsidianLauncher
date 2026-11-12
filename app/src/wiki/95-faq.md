---
title: F.A.Q
group: FAQ
summary: Short answers, each with a link to the page that covers the subject in full.
pin: faq
---

## Playing

### Can a mod get a player banned? {#will-a-mod-get-me-banned}

HELLDIVERS has no anti-cheat, and Obsidian never writes over a game file. Session mods stay out of the Galactic War, and in a public lobby they switch off while a player there doesn't allow them ([Scopes](wiki:scopes)).

### Does playing with friends who don't use Obsidian work? {#can-i-play-with-friends-who-don-t-use-obsidian}

Yes. Session mods don't change who a player finds or who can join. In a friends-only or private lobby they run with anyone, and in a public lobby they switch off while a vanilla player is there. Data overrides and older imported mods can't switch off, so a public lobby hosted with them stays closed to vanilla players. The first load into the ship shows a message that says which case applies ([Playing online](wiki:multiplayer#matchmaking)).

### Two friends both use Obsidian. Why are the session mods off? {#my-friend-and-i-both-use-obsidian-why-are-my-session-mods-off}

In a public lobby, session mods run when every other player has the same set or doesn't block session mods from other players. Setting the lobby to friends only runs them with anyone. Exporting the profile and sending the file to the friend is the quickest way to match ([Profiles](wiki:profiles#sharing-a-profile)).

### Do modded missions count for the Galactic War? {#do-my-missions-count-for-the-galactic-war}

Not while session content is deployed, and not when the content of an expansion runs in them. With only client mods and expansions they count as normal ([The Galactic War](wiki:galactic-war)). Custom wars from Commander are different: there every mission counts, session mods included.

### Is the save safe? {#is-my-save-safe}

Before a deploy, Obsidian backs up the save folder when it changed since the last backup. The first backup and the 9 newest are kept, so the save from before any mod stays ([Deploy and purge](wiki:deploy-and-purge#saves)).

### The bar says Game files changed. What now? {#the-bar-says-game-files-changed-what-now}

Some game files differ from the build Obsidian knows, usually because a mod was copied straight into the `contents` folder of the game. Deploy and Play still work. Clicking the warning shows which files differ and how to get the originals back ([Troubleshooting](wiki:troubleshooting#build-unknown)).

### What is Commander? {#what-is-commander}

The Commander tab hosts custom Galactic Wars with rules set by the host, on one PC or shared with friends over the network or the internet. The Commander subtab of this Wiki covers it ([What Commander is](wiki:commander)).

## Mods

### Do mods made before Obsidian work? {#can-i-use-mods-made-before-obsidian}

Yes. Drop the mod onto the list as usual. Obsidian keeps only the parts that differ from the game, so older mods can run together ([Adding and updating mods](wiki:adding-mods#older-mods)).

### Two mods change the same thing. Which one wins? {#two-mods-change-the-same-thing-which-one-wins}

The one lower in the list wins. Dragging a mod up or down changes the order ([Load order](wiki:load-order)).

### Is an ultrawide mod needed? {#do-i-need-an-ultrawide-mod}

No. Obsidian Patches removes the black bars on wide screens, and that option starts on. An imported ultrawide patch does the same job.

### What is Obsidian Patches? {#what-is-obsidian-patches}

A client mod that comes with Obsidian, made of small fixes that are each an option. Starting on: no intro, login screen or planet scanning, no black bars on wide screens, faster results screens after a mission, a pause that freezes a solo game, and two animation fixes, the weapon recoil after a stratagem throw and the sprint pose after a dive. Ready to switch on: skipping war briefs, an aim cursor that stays visible, and a minimap that stays open. Obsidian Patches sits in the Obsidian Kit folder at the top of the list.

## Making mods

### Which tools does making a mod take? {#do-i-need-any-tools}

A text editor. Obsidian compiles Lua when it deploys ([A first mod](wiki:first-mod)).

### Client or session? {#client-or-session}

If other players could notice the change, session. If every change happens inside content that stands apart from the normal game, such as a new mode, expansion. When in doubt, session ([Scopes](wiki:scopes)).
