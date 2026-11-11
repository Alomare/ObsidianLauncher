---
title: F.A.Q
group: FAQ
summary: Short answers about custom wars, each with a link to the page that covers the subject in full.
wiki: commander
pin: faq
---

## Wars

### Does a custom war change the official war or the Official save?

No. A custom war plays on a separate Sandbox save, in a separate matchmaking pool, and the mission results only reach the war server ([What Commander is](wiki:commander#kept-apart-from-the-official-war)).

### Do mods work in a custom war?

Yes, and missions played with session mods count there, since the results never reach the official war. In a lobby, session mods follow the usual multiplayer rules ([Playing online](wiki:multiplayer)).

### Where do rank and unlocks earned in a war go?

To the Sandbox save of that war. They stay there between seasons and don't move to the Official save ([Playing a war](wiki:commander-playing#the-war-save)).

### Does a war keep running while the host PC is off?

No. The war server runs inside Obsidian on the host PC, so the war clock freezes while Obsidian is closed there, and the war continues from the same moment at the next start ([Hosting a war](wiki:commander-hosting)).

### Can a war move to another PC?

Yes, as a copy. **Export War** on the old host and **Import War** on the new one, then players join the new host again ([Creating a war](wiki:commander-wars#export-import-and-delete)).

## Sharing and joining

### Do players who join need the Commander package?

No. Obsidian is enough: **Join War** on the Commander tab works without the package ([Joining a war](wiki:commander-joining)).

### Can friends over the internet join?

Yes. **Share**, then **Internet**, a TCP port forwarded on the router, and friends type the address and the invite code under **Join by Address**. A VPN such as Tailscale works without the port forward and without the code ([Sharing a war](wiki:commander-sharing#internet)).

### Is opening a war to the internet safe?

For friends, yes. Only war data goes over the connection, never game files or the Steam login ticket, and a caller without the invite code gets no answer. The war server trusts the player ID each game reports, though, so share the code only with people trusted not to pass it on, and use **New Code** if it leaks ([The invite code](wiki:commander-sharing#the-invite-code)).

### Can a war be open to anyone, with the address posted in public?

Not yet. A public code protects nothing, and the war server has no player sign-in, no kick or ban, and no encryption ([What is possible today](wiki:commander-sharing#what-is-possible-today)).

### How many players can a war have?

Up to 5000 players. Each mission lobby still holds four, as in the official war.
