---
title: Troubleshooting
group: Help
summary: Wars that don't show up, addresses that don't answer, the game on the wrong war, logs, and removing Commander.
wiki: commander
pin: help
---

## No war under Join War

Go through these on the host PC first. **Share** on the war screen shows **LAN** on: the switch turns off whenever Obsidian closes, so it needs switching on after every start. The Windows firewall let the war server through on private networks, and Windows treats the network as private. Both PCs are on the same network, not one on guest Wi-Fi.

When all of that holds and the list stays empty, the network may keep devices from seeing each other. Type the local address of the host under **Join by Address** instead, such as `192.168.1.20:17494`, using the IPv4 address `ipconfig` shows on the host PC and the port Share shows ([LAN](wiki:commander-sharing#lan)).

A firewall prompt answered with Cancel blocks the war server until the rule changes: in Windows Security, open **Firewall & network protection**, then **Allow an app through firewall**, and tick **Private** for `commander-server.exe`.

## "No war answered at that address"

**Join by Address** got no war back. On the host side, check that **Internet** is on in Share (it also turns off when Obsidian closes), that the router forwards the TCP port Share shows to the host PC, and that the address is the current public one. On the player side, check the port after the colon and the invite code: after **New Code**, only the new code works.

Port forwarding fails without an error when the provider shares one public address between many homes (CGNAT). The public address on the router status page then differs from the one a "what is my IP" site shows. A VPN such as Tailscale works on such connections ([VPN](wiki:commander-sharing#vpn)).

## The game shows the war as offline

The game reaches the war through Obsidian on the same PC, which then reaches the host. Check that Obsidian is open on this PC and on the host PC, and that the war still answers: the joined war tab on the Commander tab shows the host war screens while it does. After a **New Code** on the host, join again with the new code.

## The game plays the official war

The game plays a custom war only when the war is selected, the war save is the active save and the Obsidian loader is in the game. Start the game with **Play** in Obsidian, after the war shows **Selected**. A game started from Steam with nothing deployed, or started before the war was selected, plays the official war on the Official save.

## The package won't install

A downloaded package installs only with a valid signature. When the download fails the check, try **Update Package** again later, and report it on GitHub if it keeps failing. A local build installs with **Install from File**.

## Logs

The war server writes `%LOCALAPPDATA%\ObsidianLauncher\commander\server\logs\server.log` on the host PC. The war lines of the game, on every player PC, are in `loader.log`, with the other logs listed in [Troubleshooting](wiki:troubleshooting#logs) of the Launcher Wiki. **Copy report** in [Settings](app:settings) collects the versions and the end of `loader.log` for a bug report.

## Removing Commander {#removing-commander}

**Remove All Commander Data**, at the bottom of the package menu, removes the package, the hosted and joined wars, the war server data and the region names file. The dialog lists what it found first. Deleting the war saves is a choice there, and so is removing the Windows Firewall rules of the war server, which needs administrator permission and opens a Windows prompt. The war server stops first, and the next launch plays the official war.

Export any war to keep before removing, since the wars can't come back afterwards. **Remove Package** in the same menu takes only the war server and the war screens, and keeps the wars and war saves.
