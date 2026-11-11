---
title: What the guardrails can't do
group: Playing online
summary: What Obsidian protects against, and what no mod manager can stop.
---

The guardrails stop well-behaved mods from harming other players by accident, and they make the safe setup the default one. They are not anti-cheat.

## What they cover

- A vanilla player in a public lobby never plays under session mods: they switch off, and content that can't switch off keeps vanilla players out of that lobby.
- Session mods that use the loader switch off by themselves in a public lobby that didn't allow them.
- Missions played with session content aren't reported to the Galactic War.
- Achievements and account progression stay as they were whenever [Guardrails](wiki:guardrails) asks for it, whatever the mods do.
- A mod that keeps failing is switched off, and a crash at startup names the mod that was starting.

## What they can't cover

Everything runs on the PC of the player, so a person who wants to cheat can write a script that skips the loader entirely. No mod manager can prevent that. A mod can also break the rules by mistake or on purpose, by calling itself a client mod while it changes the shared game, or by changing game code directly, without the hooks of the loader, so the loader can't switch the change off.

When a mod starts, the loader compares the game before and after and writes one line per mod to the loader log, listing what the mod added or replaced outside the loader. That line is where to look when a mod behaves badly.

A mod that wrote the save files itself, from outside the game code, could get past **Freeze account progression**. Obsidian backs up the save before every deploy ([Deploy and purge](wiki:deploy-and-purge#saves)).

> [!TIP]
> Install mods from trusted authors and places, and read what a mod says it does. A client mod that changes things other players can see belongs in a report to the author or on the Discord.
