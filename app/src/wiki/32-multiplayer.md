---
title: Playing online
group: Playing online
summary: Matchmaking, public lobbies, and when session mods run in a lobby.
---

## Public lobbies {#the-multiplayer-switch}

Obsidian holds session mods back only in public lobbies, where you can end up with strangers. In a lobby set to friends only or private, everyone chose who to play with, so session mods run there with anyone, vanilla players included.

**Block session mods from other players in public lobbies** is one of the switches in [Guardrails](wiki:guardrails), at the top of the window. The switch is off by default, belongs to the active profile and applies from the next launch. With it on, other players' session mods switch off while you're in a public lobby with them. It doesn't decide whether your own session mods run.

## Matchmaking

Session mods don't change who you find or who can join you. Vanilla players see your lobby and can join it, and you can join theirs. In a public lobby, your session mods switch off while a vanilla player is there (see below).

| Your setup | Who you find in matchmaking |
|---|---|
| No session mods, or session mods that switch off live | Everyone. Your lobby shows for everyone. Expansions don't change this |
| Session mods that can't switch off live | Everyone. While your lobby is public, only Obsidian players can join it, and you can join another public lobby only when its host runs such content too |
| Hosting the content of an expansion | Your lobby shows only for players who run that expansion, at the same version |

With an expansion deployed, you also see the lobbies of players running the content of that same expansion, wherever your own matchmaking puts you.

The first time you load into the ship, a message says which case applies to you: your mods don't affect other players, your session mods switch off in some public lobbies, some of your content keeps players without Obsidian out of your public lobby, or an expansion may keep you apart from players who don't have it while its content runs.

## When session mods run

Alone, and in a friends-only or private lobby, your session mods always run. In a public lobby they run only when every other player either:

1. runs the same set of session mods, with the same versions and the same option values, or
2. runs Obsidian and doesn't block other players' session mods.

Client mods always run.

A few examples, from the point of view of player A with session mods, in a public lobby:

| Other players | A's session mods |
|---|---|
| B runs the same set | On |
| B runs other session mods, not blocking | On |
| B runs other session mods, blocking | Off |
| B runs only client mods, not blocking | On |
| B has vanilla HELLDIVERS | Off |
| B not blocking, C blocking with a different set | Off: everyone has to allow them |

In a friends-only or private lobby, every row is On.

The check runs again whenever someone joins, leaves or changes setup, and when the host changes who can join. Mods switch on and off live, and a session mod can undo what it changed when that happens.

## Mods with parts

A [mod with parts](wiki:scopes#mods-with-parts) follows the same rules, part by part. The session parts run where session mods run, and the client parts run in any lobby. When the session parts switch off in a public lobby, the game names them and says the rest keeps working.

| Other players | A's camera part (client) | A's weapon range part (session) |
|---|---|---|
| B runs the same set | On | On |
| B has vanilla HELLDIVERS | On | Off |

The options of client parts don't count when Obsidian compares sets, and a part switched off in the mod's options counts for nothing.

## Mods every player needs

Some mods only work when everyone has them, such as one that lets more players into a lobby. The author marks those, and they follow a stricter rule: such a mod runs only when every other player runs it too, at the same version and with the same option values, in any lobby, friends only included. Not blocking doesn't count for them. When a player who doesn't have it is in your lobby, that one mod switches off and the game tells you. Your other session mods follow the usual rules.

> [!TIP]
> Option values marked as personal by the author, such as a camera distance, don't count when Obsidian compares sets. A friend can pick a different camera distance and still match you.

## Expansions

An [expansion](wiki:scopes#expansions) runs as a client mod until the host starts the content. The start checks every player in the lobby: each has to run the same expansion, at the same version and with the same option values. If someone doesn't, nothing starts and the host is told who. The Guardrails switch doesn't count here.

Once the content starts, every game in the lobby follows the host. A player who tries to join without the expansion is turned away, and the host sees the name. Nobody joining or leaving switches the content off for the players already in it. When the host stops the content, or you leave the lobby, the expansion is a client mod again.

## Mods that can't switch off live

Some session content can't be switched off in the middle of a game: data overrides and older imported mods. While a player with such content hosts a public lobby, vanilla players can't join it and the game tells them the version doesn't match. That player also can't take the content into a public lobby whose host doesn't run such content. Friends-only and private lobbies are open to everyone. If a player who blocks session mods ends up in such a public lobby, the loader records a warning in `loader.log` and the session mods that can switch off stay off. Leave the lobby, or play in a friends-only lobby.

## Invites

Steam invites and joining friends may cross matchmaking pools. The rules above still decide which mods run once you are in.
