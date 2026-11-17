---
title: Guardrails
group: Playing online
summary: The shield at the top: session mods from other players, achievements, account progression and the lobby list.
---

**Guardrails**, the shield button in the bar at the top of the window, holds four switches for the active profile. They apply from the next launch of the game, and to everything that runs in it: no mod can change them or get past them. All four start off.

## Session mods of other players

**Block session mods from other players in public lobbies** keeps the session mods of other players out of the game while in a public lobby with them. The switch is off by default, so playing with other Obsidian players just works, and friends-only and private lobbies are never blocked. It doesn't decide whether the session mods of the player who set it run ([Playing online](wiki:multiplayer)).

## Achievements

**Keep achievements from unlocking** stops every Steam achievement during play. The progress the game counts toward one isn't kept either, so nothing pops later in a normal game.

## Frozen progression

**Freeze account progression** keeps XP and rank, samples, unlocks, upgrades and stats as they were when the game started. Missions, the Armory and mods can still change them during play, but none of it is saved, and the next launch starts from the same account. Settings and looks, such as keys or the helmet, still save. The switch is for trying mods, the Trainer or a new game mode without any of it counting.

> [!NOTE]
> Buying something in the Armory with this on looks like it works, and is undone at the next save.

## The lobby list

A lobby hosted with Obsidian lists the mods it runs in the multiplayer terminal of the game ([The Modded tab](wiki:multiplayer#the-modded-tab)). Session mods and expansions are always listed, because they change the game of the other players. **Hide client mods from the lobby list** leaves client mods out of that list.

## In the app

The shield fills while **Block session mods from other players in public lobbies**, **Keep achievements from unlocking** or **Freeze account progression** is on, and hovering over it shows the state of those three. In the profiles list, a shield marks a profile with achievements or progression guarded, and the players icon marks one that blocks session mods from other players.

A shared or imported profile keeps the achievements and progression switches, and Obsidian says so when an imported one has them on. **Block session mods from other players in public lobbies** is left off in an imported profile, for each player to choose.

## Always on

Galactic War results and leaderboards are never sent while session mods are deployed, or from a mission the content of an expansion ran in, and Proving Grounds stays closed then ([The Galactic War](wiki:galactic-war)). That protects other players, so it isn't a choice. [What the guardrails can't do](wiki:honest-limits) covers the limits of all of this.

Mod authors can read the first three switches with `obsidian.guardrails()` ([Loader API](wiki:loader-api#obsidian-guardrails)).
