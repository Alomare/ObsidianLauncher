---
title: "Scopes: client and session"
group: Playing online
summary: Why every mod is either client or session, and what each one may do.
---

HELLDIVERS is a co-op game. A mod that changes enemies, weapons or missions changes the game for everyone in your lobby, not only for you. Obsidian sorts every mod into one of two scopes so that a player who never installed a mod is never affected by one.

## Client mods

A client mod changes only what you see, hear or press: the camera, the HUD, input, menus, local visuals and sound. Nothing it does reaches the other players. Client mods run everywhere, in any lobby, with anyone.

Obsidian Quality of Life is a client mod.

## Session mods

A session mod can change the shared game: balance, enemies, spawns, missions, physics, anything other players would notice. Session mods only run in a lobby where every other player has agreed to them ([Playing online](wiki:multiplayer)). Alone, they always run.

## Unspecified mods

A mod that doesn't declare a scope is unspecified, and so is an older mod imported from whole bundles, because Obsidian can't know what it changes. Unspecified mods run as client mods, in any lobby: being kept out of multiplayer by a guess would be worse. If one does change the shared game, the other players may notice, so play it with people who have it too.

A mod that asks to run only while every player runs it (`every_player`) is a session mod, scope or not.

## Libraries

A library is code other mods build on, such as Obsidian API. It takes the scope of the mods using it: a session mod while an enabled session mod requires it, and a client mod the rest of the time. A library that no mod uses, or that only client mods use, doesn't move you out of the normal matchmaking pool or stop your Galactic War results.

## In the app

Every row of the mod list has a diamond for the scope, green for client, orange for session, blue for a library and grey for unspecified. Hover over it for the name. The details of each mod say it too.

## Who decides the scope

The author of the mod declares it in the `obsidian.json` file. Obsidian then enforces it: the hooks of a session mod switch off by themselves when the lobby isn't cleared, so the author doesn't have to write any of that ([Writing session mods](wiki:session-mods)).

> [!WARNING]
> A scope is a promise from the author. A mod that calls itself client but changes the shared game breaks the rules, and Obsidian can't always tell. [What the guardrails can't do](wiki:honest-limits) covers this.
