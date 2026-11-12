---
title: Obsidian Trainer
group: Getting started
summary: The in-game menu that changes almost anything.
---

Obsidian Trainer is an in-game menu for the character, weapons, stratagems, enemies, missions and game data. It comes with Obsidian and starts off.

## Turning it on

Switch on **Obsidian Trainer** in the Mods list and deploy. Obsidian API, the mod the Trainer is built on, turns on with it. In the game, **F6** opens the menu, and the Trainer pages are in the **Obsidian Trainer** folder on the left. The window and the key come from Obsidian API, so pages of other mods show there too, each mod in a separate folder, and the Keys tab of Obsidian API changes the key.

| Key | What it does |
|---|---|
| Arrows, or W A S D | Move, and change a value |
| Enter, or E | Select, or type a number |
| Shift with left or right | Change a number ten steps at a time |
| Tab | Filter the list |
| R | Reset the selected value |
| Esc, or Q | Go back, and close |

The bottom of the menu shows these keys as keycaps. A row being typed into shows the text in place, and a stratagem code row takes the arrow keys or W A S D, with Backspace to take one back. The menu also works with the mouse, with the normal system cursor:

| Mouse | What it does |
|---|---|
| Point at a row | Select it |
| Click a row | Use it |
| Click `<` or `>` | Change a value, and hold to keep changing it |
| Click a number | Type a value |
| Click the arrow at the end of a row | Reset the value, once it has changed |
| Wheel, or the bar at the side | Scroll the list |
| Right click | Go back |
| Title bar | Drag to move the window |
| Bottom right corner | Drag to resize the window |

The window opens where it was last left, at the same size. On the list, Enter or a click opens and closes a folder, and each folder stays as it was left.

The menu blocks the character while open and closes by itself when the game changes screens, such as at the start of a mission. A gamepad can move around the menu, but only F6 opens it.

## What is in it

- **Player:** god mode, infinite sprint, speed, teleporting, respawning, and owned helmets, armor and capes.
- **Weapons:** infinite ammo, instant reload, refills, the fire rate, damage, magazine, reload, spread and armor piercing of each weapon the player holds, and any weapon, backpack or perk from the base game or an owned DLC, with the chosen upgrades.
- **Stratagems:** no cooldowns, unlimited uses, instant call-ins, ignoring no-stratagem zones, any stratagem from the base game or an owned DLC dropped at the cursor, and real flares, Reinforce included. **New stratagem** builds a stratagem from one in the game: a name, code, icon, cooldown, uses and call-in time, and what happens when it lands. Other players see the stratagem it started from, and it comes back at the next launch.
- **Vehicles:** speed, turning, armor, no damage and endless exosuit rockets for the vehicle being driven, plus repairs and refills.
- **Bots:** Helldivers played by this machine, with a HUD slot or as bare bodies past four players. Orders for every bot or each one: follow the player, hold, walk to the cursor, come to the player, fight, face the cursor, shoot, throw, reload, enter a stratagem code the way a player does and throw the flare at the cursor, and call stratagems, plus weapons, backpacks, perks and stratagems to give them. Host only, in a mission.
- **Spawn:** ammunition, supply crates and pickups on the spot, and flares at the cursor.
- **Enemies:** stopping spawns, freezing, blinding or stunning enemies, killing them, spawning more, and setting how hard encounters and patrols push.
- **Mission:** the objective list, finishing objectives, protecting convoys and escorts, fast extraction, skipping waits, Proving Grounds modifiers in any mission, the players in the lobby, research samples, and custom missions of any kind from the ship, which pay XP and samples and leave the campaign alone.
- **World:** game speed, camera, HUD, fog, time of day and gravity, and the sound: volumes, and a **Soundtrack** page that plays any music track the screen has, silences or pauses it, and gives it back to the game.
- **Editor:** the numbers of every weapon, grenade, backpack, stratagem, sentry, mine, strike, vehicle, enemy and status effect.
- **Console:** the developer commands.
- **Changes:** everything switched on or edited, with a way to undo it all at once.

Every option has a description at the bottom of the menu while selected. Spawning stops once the map holds 64 enemies, corpses included, because far more can crash the game. Actions only the host can do say so when chosen.

## What it doesn't do

- **No progression edits.** No page of the Trainer writes the save. What a mission with it on earns counts, as in any other. [Guardrails](wiki:guardrails) can keep a launch from counting.
- **No DLC the player doesn't own.** Lists show all of the base game, whatever has been unlocked by playing, plus owned DLC. Items last for that mission only, so nothing is added to the save.
- **No changes to the shared war.** Commands that report to the Galactic War are refused, and results stay blocked ([The Galactic War](wiki:galactic-war)).

> [!WARNING]
> The game has an anti-tamper check that cuts a player off from the network until the game restarts. Obsidian Trainer changes guarded values only the way the game itself does, so it shouldn't trigger. Other tools and hand-made scripts can.

## Playing with others

The Trainer is a session mod, so in a public lobby everyone has to run the same mods or allow session mods ([Playing online](wiki:multiplayer)). When someone joins who doesn't, every change is undone and the Trainer pages close until that player leaves, and then the changes come back. In a friends-only or private lobby the Trainer always runs. Numbers from the Editor and the two damage rows run on the game of each player, so they work best when everyone uses the same ones.

Two things need a lobby with one player: giving laser sight, because the games of other players would run it with no setup, and `spawn_enemy` in the console, because it loads a package on one game only.

## Keeping changes

Changes last until the game closes. In **Changes**, **Keep edits for later launches** saves the Editor edits, and they come back at each launch while the **Keep saved edits** option of the mod is on.
