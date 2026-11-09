---
title: Guardrails
group: Playing online
summary: The shield at the top: other players' session mods, achievements and your account's progression.
---

**Guardrails**, the shield at the top of the window, holds three switches for the active profile. They apply from the next time you launch the game, and to everything that runs in it: no mod can change them or get past them.

## Session mods of other players

**Block session mods from other players in public lobbies** keeps other players' session mods out of your game while you're in a public lobby with them. The switch is off by default, so playing with other Obsidian players just works. Friends-only and private lobbies are never blocked. It doesn't decide whether your own session mods run ([Playing online](wiki:multiplayer)).

## Achievements

**Keep achievements from unlocking** stops every Steam achievement while you play. The progress the game counts toward one isn't kept either, so nothing pops later in a normal game.

## Frozen progression

**Freeze account progression** keeps your XP and rank, samples, unlocks, upgrades and stats as they were when the game started. Missions, the Armory and mods can still change them while you play, but none of it is saved, and the next launch starts from the same account. Settings and looks, such as your keys or your helmet, still save.

It's for trying mods, the trainer or a new game mode without it counting.

> [!NOTE]
> Buying something in the Armory with this on looks like it works, and is undone at the next save.

## In the app

The shield fills, with a number, while any of the switches is on. Hover over it to see all three. In the profiles list, a shield marks a profile with either of the last two on, and the players icon one that blocks other players' session mods.

A profile you share or import keeps the last two, and Obsidian tells you when an imported one has them on. **Block session mods from other players in public lobbies** is left off in an imported profile, for each player to choose.

## Always on

Galactic War results and leaderboards are never sent while session mods are deployed, or from a mission the content of an expansion ran in, and Proving Grounds stays closed then ([The Galactic War](wiki:galactic-war)). That protects other players, so it isn't a choice. [What the guardrails can't do](wiki:honest-limits) covers the limits of all of this.

Mod authors can read the player's choices with `obsidian.guardrails()` ([Loader API](wiki:loader-api#obsidian-guardrails)).
