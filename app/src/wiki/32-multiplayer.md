---
title: Playing online
group: Playing online
summary: Matchmaking, public lobbies, and when session mods run in a lobby.
---

## Public lobbies {#the-multiplayer-switch}

Obsidian holds session mods back only in public lobbies, where strangers can join. In a lobby set to friends only or private, everyone chose who to play with, so session mods run there with anyone, vanilla players included.

**Block session mods from other players in public lobbies** is one of the switches in [Guardrails](wiki:guardrails), at the top of the window. The switch is off by default, belongs to the active profile and applies from the next launch. With it on, the session mods of other players switch off while in a public lobby with them. It doesn't decide whether the session mods of the player who set it run.

## Matchmaking

Session mods don't change who a player finds or who can join. Vanilla players see a modded lobby and can join it, and the other way round. In a public lobby, session mods switch off while a vanilla player is there (see below).

| Setup | Who matchmaking finds |
|---|---|
| No session mods, or session mods that switch off live | Everyone. The lobby shows for everyone, and expansions don't change this |
| Session mods that can't switch off live | Everyone. While the lobby is public, only Obsidian players can join it, and joining another public lobby works only when the host there runs such content too |
| Hosting the content of an expansion | The lobby shows only for players who run that expansion, at the same version |

With an expansion deployed, the lobbies of players running the content of that same expansion show too, wherever matchmaking puts the player otherwise.

The first load into the ship shows a message that says which case applies: the mods don't affect other players, the session mods switch off in some public lobbies, some content keeps players without Obsidian out of a public lobby, or an expansion may keep the player apart from players who don't have it while the content runs.

## When session mods run

Alone, and in a friends-only or private lobby, session mods always run. In a public lobby they run only when every other player either:

1. runs the same set of session mods, with the same versions and the same option values, or
2. runs Obsidian and doesn't block session mods from other players.

Client mods always run. A few examples, from the point of view of player A with session mods, in a public lobby:

| Other players | Session mods of A |
|---|---|
| B runs the same set | On |
| B runs other session mods, not blocking | On |
| B runs other session mods, blocking | Off |
| B runs only client mods, not blocking | On |
| B has vanilla HELLDIVERS | Off |
| B not blocking, C blocking with a different set | Off: everyone has to allow them |

In a friends-only or private lobby, every row is On. The check runs again whenever someone joins, leaves or changes setup, and when the host changes who can join. Mods switch on and off live, and a session mod can undo what it changed when that happens.

## Hybrid Scopes

A mod with [Hybrid Scopes](wiki:scopes#hybrid-scopes) follows the same rules, one secondary scope at a time. The session secondary scopes run where session mods run, and the client ones run in any lobby. When the session secondary scopes switch off in a public lobby, the game names them and says the rest keeps working.

| Other players | Camera of A (client secondary scope) | Weapon range of A (session secondary scope) |
|---|---|---|
| B runs the same set | On | On |
| B has vanilla HELLDIVERS | On | Off |

The options of client secondary scopes don't count when Obsidian compares sets, and a secondary scope switched off in the options of the mod counts for nothing.

## Mods every player needs

Some mods only work when everyone has them, such as one that lets more players into a lobby. The author marks those, and they follow a stricter rule: such a mod runs only when every other player runs it too, at the same version and with the same option values, in any lobby, friends only included. Not blocking doesn't count for them. While a player without it is in the lobby, that one mod switches off and the game says so, and the other session mods follow the usual rules.

> [!TIP]
> Option values the author marks as personal, such as a camera distance, don't count when Obsidian compares sets. A friend with a different camera distance still matches.

## Expansions

An [expansion](wiki:scopes#expansions) runs as a client mod until the host starts the content. The start checks every player in the lobby: each has to run the same expansion, at the same version and with the same option values. If someone doesn't, nothing starts and the host learns who. The Guardrails switch doesn't count here.

Once the content starts, every game in the lobby follows the host. A player who tries to join without the expansion is turned away, and the host sees the name. Nobody joining or leaving switches the content off for the players already in it. When the host stops the content, or a player leaves the lobby, the expansion is a client mod again for that player.

## Mods that can't switch off live

Some session content can't switch off in the middle of a game: data overrides and older imported mods. While a player with such content hosts a public lobby, vanilla players can't join it, and the game tells them the version doesn't match. That player also can't take the content into a public lobby whose host doesn't run such content. Friends-only and private lobbies are open to everyone.

When a player who blocks session mods ends up in such a public lobby, the loader records a warning in `loader.log`, and the session mods that can switch off stay off. Leave the lobby, or play in a friends-only lobby.

## Invites

Steam invites and joining friends may cross matchmaking pools. The rules above still decide which mods run once the player is in the lobby.
