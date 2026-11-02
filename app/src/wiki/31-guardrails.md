---
title: Guardrails
group: Playing online
summary: The shield at the top: other players' session mods, achievements and your account's progression.
---

**Guardrails**, the shield at the top of the window, holds three switches for the active profile. They apply from the next time you launch the game, and to everything that runs in it: no mod can change them or get past them.

## Session mods of other players

**Allow other players' session mods in my game** lets other Obsidian players run their session mods in your game when you play together. It doesn't decide whether your own session mods run ([Playing online](wiki:multiplayer)).

## Achievements

**Keep achievements from unlocking** stops every Steam achievement while you play. The progress the game counts toward one isn't kept either, so nothing pops later in a normal game.

## Frozen progression

**Freeze my account's progression** keeps your XP and rank, samples, unlocks, upgrades and stats as they were when the game started. Missions, the Armory and mods can still change them while you play, but none of it is saved, and the next launch starts from the same account. Settings and looks, such as your keys or your helmet, still save.

It's for trying mods, the trainer or a new game mode without it counting.

> [!NOTE]
> Buying something in the Armory with this on looks like it works, and is undone at the next save.

## In the app

The shield fills, with a number, while either of the last two switches is on. Hover over it to see all three. In the profiles list, a shield marks a profile that has either of them on.

A profile you share or import keeps those two, and Obsidian tells you when an imported one has them on. **Allow other players' session mods in my game** is left off in an imported profile, for each player to choose.

## Always on

Galactic War results and leaderboards are never sent while session mods are deployed, and Proving Grounds stays closed ([The Galactic War](wiki:galactic-war)). That protects other players, so it isn't a choice. [What the guardrails can't do](wiki:honest-limits) covers the limits of all of this.

Mod authors can read the player's choices with `obsidian.guardrails()` ([Loader API](wiki:loader-api#obsidian-guardrails)).
