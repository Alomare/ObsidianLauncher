---
title: Scopes
group: Playing online
summary: Client, session, expansion, library and unspecified mods, Hybrid Scopes, and what each one may do.
---

HELLDIVERS is a co-op game. A mod that changes enemies, weapons or missions changes the game for everyone in the lobby, not only for the player who installed it. Obsidian sorts every mod into a scope, so that a player who never installed a mod is never affected by one.

## Client mods

A client mod changes only what one player sees, hears or presses: the camera, the HUD, input, menus, local visuals and sound. Nothing it does reaches the other players, so client mods run everywhere, in any lobby, with anyone. Obsidian Patches is a client mod.

## Session mods

A session mod can change the shared game: balance, enemies, spawns, missions, physics, anything other players would notice. In a public lobby, session mods only run when every other player allows them. Alone, and in a friends-only or private lobby, they always run ([Playing online](wiki:multiplayer)).

A session mod changes the whole deploy, in every mission: Galactic War and leaderboard results aren't sent, Proving Grounds is closed, and matchmaking finds only other Obsidian players ([The Galactic War](wiki:galactic-war)).

## Expansions

An expansion extends the main game and leaves vanilla unaffected. It adds content that stands apart from the normal game, such as an arcade mode reached from a separate mission screen on the ship, and changes nothing outside that content.

Outside the content, an expansion counts as a client mod. Matchmaking stays normal, Galactic War results are sent, and Proving Grounds stays open. The screens and menu pages that lead to the content show in any lobby, and nothing else the expansion does runs.

The host starts the content. Obsidian first checks that every player in the lobby runs the same expansion, at the same version and with the same option values. If someone doesn't, the start is refused and the host learns who. Otherwise every game in the lobby follows the host into the content, and while it runs:

- the expansion can change the shared game, as a session mod does;
- the Galactic War and leaderboard results of that mission aren't sent, and Proving Grounds is closed;
- the lobby moves to a matchmaking pool for that expansion, where players who run it at the same version find the lobby;
- a player who tries to join without the expansion is turned away before reaching the mission.

When the content ends, or when the player leaves the lobby, the expansion is a client mod again.

## Libraries

A library is code other mods build on, such as Obsidian API. It takes the scope of the mods using it: a session mod while an enabled session or hybrid mod requires it, and a client mod the rest of the time. A library that no mod uses, or that only client mods and expansions use, keeps the normal matchmaking pool and the Galactic War results. An expansion gets full use of the libraries it requires while the content runs.

## Hybrid Scopes

A mod can run with more than one scope. The scope the author declares in `scope` is the primary scope: an older Obsidian reads only that one, and runs the whole mod with it. Secondary scopes hold the code, options and files of the mod that behave differently from the primary scope.

A session mod with a client secondary scope keeps that code running in any lobby. A camera mod, for example, has session as the primary scope, keeps the camera in a client secondary scope and puts a longer weapon range in a session secondary scope. In a lobby that isn't cleared, the session secondary scopes switch off, the client ones keep running, and the game says which ones went off.

An expansion with a session secondary scope also runs that code outside the content, in a cleared lobby, and switches it off in a lobby that isn't cleared. Inside the content of the expansion, everything runs as before.

A secondary scope can have an option in the details of the mod that switches it on and off. While a session secondary scope is on, the mod counts as a session mod for the deploy: Galactic War results aren't sent and Proving Grounds is closed. With every session secondary scope off, a session mod deploys as a client mod and an expansion as an expansion, and the Galactic War and Proving Grounds are back after the next deploy.

A session mod with a client and a session secondary scope both on is **hybrid**.

## Unspecified mods

A mod that doesn't declare a scope is unspecified, and so is an older mod imported from whole bundles, because Obsidian can't know what it changes. Unspecified mods run as client mods in any lobby, since keeping a player out of multiplayer on a guess would be worse. When one does change the shared game, the other players may notice, so play it with people who have it too.

A mod that asks to run only while every player runs it (`every_player`) is a session mod, whatever scope it declares.

## In the app

Every row of the mod list has a diamond for the scope: black for unspecified, green for client, red for session, white for a library (grey on the light theme) and yellow for an expansion. A hybrid mod has a diamond split in two, green on the left and red on the right. Hovering over it shows the name of the scope and what it means, with a WIKI tag: a click opens this page at that scope. The details of each mod name the scope too, and the About tab lists the secondary scopes, marking one that is off.

## Who decides the scope

The author of the mod declares it in the `obsidian.json` file, and Obsidian enforces it. The hooks of a session mod switch off by themselves when the lobby isn't cleared, and the hooks of an expansion stay off until the content starts, so the author writes none of that ([Writing session mods](wiki:session-mods)).

> [!WARNING]
> A scope is a promise from the author. A mod that calls itself client but changes the shared game breaks the rules, and Obsidian can't always tell. [What the guardrails can't do](wiki:honest-limits) covers this.
