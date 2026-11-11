---
title: Hosting a war
group: Hosting and sharing
summary: What the host PC does, what stops when Obsidian closes, and where the wars are kept.
wiki: commander
---

## The host PC

The PC with the Commander package hosts every war created on it. The war server runs inside Obsidian there, answers the games of every player in the war and keeps the war clock. The host can play in the same war from the same PC like any other player.

Every war needs that PC on and Obsidian open on it. When Obsidian closes, the war server stops with it: the war clock freezes, the games of the other players show the war as offline, and **Share** closes again. After the next start, open **Share** and switch **LAN** and **Internet** back on. The invite code stays the same, so players don't need a new one.

## Share

**Share** on the war screen opens the sharing options of that war: **LAN** for players on the same network, **Internet** for players elsewhere, and the invite code that Internet needs. [Sharing a war](wiki:commander-sharing) covers each one. A chip beside the season shows when a war is open to LAN or to the internet.

The first time a war opens to the network, Windows asks whether the war server may get through the firewall. Allow it on private networks, or the PCs on the network can't reach the war. When Windows lists the home network as public, set the network profile to private in the Windows network settings first. Obsidian never creates firewall rules by itself, and **Remove All Commander Data** can remove the rule again.

## Where wars are kept

The wars of the host, with history and statistics, live in `%LOCALAPPDATA%\ObsidianLauncher\commander\server`. **Export War** on the war screen saves one war as a single file, which is the way to back a war up or move it to another PC.

A war has one host at a time. Two PCs can't share the hosting, and a war imported on another PC starts a separate copy that players join again.
