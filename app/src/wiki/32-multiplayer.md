---
title: Playing online
group: Playing online
summary: Matchmaking, the multiplayer switch, and when session mods run in a lobby.
---

## The multiplayer switch

**Allow other modders to affect my game** sits at the top of the window. It belongs to the active profile. Switching it on says one thing: other Obsidian players may run their session mods in your game.

It does not decide whether your own session mods run. That depends on the other players, as described below.

## Matchmaking

Obsidian changes which public lobbies you can find, so a vanilla player never ends up in a modded game by chance.

| Your setup | Who you find in matchmaking |
|---|---|
| No session mods, switch off | Everyone. This is normal play. |
| Session mods deployed, or switch on | Only other Obsidian players with session mods or the switch on |
| A mod every player needs deployed | Only players who run that same mod, at the same version |

So with the switch on, your lobby shows up for players who run session mods, and you no longer see vanilla lobbies. Turn it off when you want normal matchmaking back.

The lobby filters in the multiplayer terminal say **(Obsidian Only)** while you are in the Obsidian pool, so an empty list doesn't look like a broken one.

## When session mods run

In a lobby, your session mods run only when every other player either:

1. runs the same set of session mods, with the same versions and the same option values, or
2. has the multiplayer switch on.

Alone, your session mods always run. Client mods always run.

A few examples, from the point of view of player A with session mods:

| Other players | A's session mods |
|---|---|
| B runs the same set | On |
| B runs other session mods, switch on | On |
| B runs other session mods, switch off | Off |
| B runs no mods, switch on | On |
| B has vanilla HELLDIVERS | Off |
| B on, C off and on a different set | Off: everyone has to agree |

The check runs again whenever someone joins, leaves or changes setup. Mods switch on and off live, and a session mod can undo what it changed when that happens.

## Mods every player needs

Some mods only work when everyone has them, such as one that lets more players into a lobby. The author marks those, and they follow a stricter rule: such a mod runs only when every other player runs it too, at the same version and with the same option values. The multiplayer switch doesn't count for them. When a player who doesn't have it is in your lobby, that one mod switches off and the game tells you. Your other session mods follow the usual rules.

These mods also get a matchmaking pool of their own, so you only find players who run the same ones.

> [!TIP]
> Option values marked as personal by the author, such as a camera distance, don't count when Obsidian compares sets. A friend can pick a different camera distance and still match you.

## Mods that can't switch off live

Some session content can't be switched off in the middle of a game: data overrides and older imported mods. If a player who hasn't agreed ends up in such a lobby, for example through an invite, the loader records a warning in `loader.log` and the session mods that can switch off stay off. Leave the lobby, or play with players who agreed.

## Invites

Steam invites and joining friends may cross matchmaking pools. The rules above still decide which mods run once you are in.
