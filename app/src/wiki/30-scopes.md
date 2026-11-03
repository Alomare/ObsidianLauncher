---
title: Scopes
group: Playing online
summary: Client, session, expansion, library and unspecified mods, and what each one may do.
---

HELLDIVERS is a co-op game. A mod that changes enemies, weapons or missions changes the game for everyone in your lobby, not only for you. Obsidian sorts every mod into a scope so that a player who never installed a mod is never affected by one.

## Client mods

A client mod changes only what you see, hear or press: the camera, the HUD, input, menus, local visuals and sound. Nothing it does reaches the other players. Client mods run everywhere, in any lobby, with anyone.

Obsidian Quality of Life is a client mod.

## Session mods

A session mod can change the shared game: balance, enemies, spawns, missions, physics, anything other players would notice. Session mods only run in a lobby where every other player has agreed to them ([Playing online](wiki:multiplayer)). Alone, they always run.

A session mod changes the whole deploy, in every mission: Galactic War and leaderboard results aren't sent, Proving Grounds is closed, and matchmaking finds only other Obsidian players ([The Galactic War](wiki:galactic-war)).

## Expansions

An expansion extends the main game and leaves vanilla unaffected. It adds content that stands apart from the normal game, such as an arcade mode reached from a separate mission screen on the ship, and changes nothing outside that content.

Outside the content, an expansion counts as a client mod. Matchmaking stays normal, Galactic War results are sent, and Proving Grounds stays open. The screens and menu pages that lead to the content show in any lobby, and nothing else the expansion does runs.

The host starts the content. Obsidian first checks that every player in the lobby runs the same expansion, at the same version and with the same option values. If someone doesn't, the start is refused and the host is told who. Otherwise every game in the lobby follows the host into the content, and while it runs:

- the expansion can change the shared game, as a session mod does;
- the Galactic War and leaderboard results of that mission aren't sent, and Proving Grounds is closed;
- the lobby moves to a matchmaking pool for that expansion, where players who run it at the same version find the lobby;
- a player who tries to join without the expansion is turned away before reaching the mission.

When the content ends, or when you leave the lobby, the expansion is a client mod again.

## Unspecified mods

A mod that doesn't declare a scope is unspecified, and so is an older mod imported from whole bundles, because Obsidian can't know what it changes. Unspecified mods run as client mods, in any lobby: being kept out of multiplayer by a guess would be worse. If one does change the shared game, the other players may notice, so play it with people who have it too.

A mod that asks to run only while every player runs it (`every_player`) is a session mod, scope or not.

## Libraries

A library is code other mods build on, such as Obsidian API. It takes the scope of the mods using it: a session mod while an enabled session mod requires it, and a client mod the rest of the time. A library that no mod uses, or that only client mods and expansions use, doesn't move you out of the normal matchmaking pool or stop your Galactic War results. An expansion gets full use of the libraries it requires while the content runs.

## In the app

Every row of the mod list has a diamond for the scope: black for unspecified, green for client, red for session, white for a library and yellow for an expansion. Hover over it for the name. The details of each mod say it too.

## Who decides the scope

The author of the mod declares it in the `obsidian.json` file. Obsidian then enforces it: the hooks of a session mod switch off by themselves when the lobby isn't cleared, and the hooks of an expansion stay off until the content starts, so the author doesn't have to write any of that ([Writing session mods](wiki:session-mods)).

> [!WARNING]
> A scope is a promise from the author. A mod that calls itself client but changes the shared game breaks the rules, and Obsidian can't always tell. [What the guardrails can't do](wiki:honest-limits) covers this.
