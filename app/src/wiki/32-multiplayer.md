---
title: Playing online
group: Playing online
summary: Matchmaking, public lobbies, and when session mods run in a lobby.
---

## Public lobbies {#the-multiplayer-switch}

Obsidian holds session mods back only in public lobbies, where you can end up with strangers. In a lobby set to friends only or private, everyone chose who to play with, so session mods run there with anyone, vanilla players included.

**Block session mods from others in my public lobby** is one of the switches in [Guardrails](wiki:guardrails), at the top of the window. The switch is off by default, belongs to the active profile and applies from the next launch. With it on, other players' session mods switch off while you're in a public lobby with them. It doesn't decide whether your own session mods run.

## Matchmaking

Session mods don't change who you find or who can join you. Vanilla players see your lobby and can join it, and you can join theirs. In a public lobby, your session mods switch off while a vanilla player is there (see below).

| Your setup | Who you find in matchmaking |
|---|---|
| No session mods, or session mods that switch off live | Everyone. Your lobby shows for everyone. Expansions don't change this |
| Session mods that can't switch off live | Everyone. While your lobby is public, only Obsidian players can join it, and you can join another public lobby only when its host runs such content too |
| A mod every player needs deployed | Only players who run that same mod, at the same version |
| Hosting the content of an expansion | Your lobby shows only for players who run that expansion, at the same version |

With an expansion deployed, you also see the lobbies of players running the content of that same expansion, wherever your own matchmaking puts you.

The lobby filters in the multiplayer terminal say **(Obsidian Only)** while the list shows only Obsidian players, so an empty list doesn't look like a broken one.

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

## Mods every player needs

Some mods only work when everyone has them, such as one that lets more players into a lobby. The author marks those, and they follow a stricter rule: such a mod runs only when every other player runs it too, at the same version and with the same option values, in any lobby, friends only included. Not blocking doesn't count for them. When a player who doesn't have it is in your lobby, that one mod switches off and the game tells you. Your other session mods follow the usual rules.

These mods also get a matchmaking pool of their own, so you only find players who run the same ones.

> [!TIP]
> Option values marked as personal by the author, such as a camera distance, don't count when Obsidian compares sets. A friend can pick a different camera distance and still match you.

## Expansions

An [expansion](wiki:scopes#expansions) runs as a client mod until the host starts the content. The start checks every player in the lobby: each has to run the same expansion, at the same version and with the same option values. If someone doesn't, nothing starts and the host is told who. The Guardrails switch doesn't count here.

Once the content starts, every game in the lobby follows the host. A player who tries to join without the expansion is turned away, and the host sees the name. Nobody joining or leaving switches the content off for the players already in it. When the host stops the content, or you leave the lobby, the expansion is a client mod again.

## Mods that can't switch off live

Some session content can't be switched off in the middle of a game: data overrides and older imported mods. While a player with such content hosts a public lobby, vanilla players can't join it and the game tells them the version doesn't match. That player also can't take the content into a public lobby whose host doesn't run such content. Friends-only and private lobbies are open to everyone. If a player who blocks session mods ends up in such a public lobby, the loader records a warning in `loader.log` and the session mods that can switch off stay off. Leave the lobby, or play in a friends-only lobby.

## Invites

Steam invites and joining friends may cross matchmaking pools. The rules above still decide which mods run once you are in.
