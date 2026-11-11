---
title: Sharing a war
group: Hosting and sharing
summary: LAN, VPN and internet play, the invite code, and what is possible today for a war shared outside the home network.
wiki: commander
---

## Three ways to share

Only the war data crosses the network: the map state once a minute, a report after each mission, and the war screens. Game traffic between players still goes through Steam as always, so sharing a war changes nothing about lobbies or connection quality.

| Way | For | Host setup | Players type |
|---|---|---|---|
| LAN | Players on the same home or local network | **LAN** in Share | Nothing: the war shows under **Join War** |
| VPN | Friends on a VPN such as Tailscale or ZeroTier | **LAN** in Share, everyone on the same VPN | The VPN address of the host, with the port |
| Internet | Friends anywhere | **Internet** in Share, a port forwarded on the router | The public address of the host with the port, and the invite code |

## LAN

**Share** on the war screen, then **LAN**. The war server starts listening on the network, on port 17494 or the next free one, and announces the war every few seconds. Obsidian on any PC of the same network lists it under **Join War**, with the host computer name, the season and the player count. Players on the network need no code.

Some networks keep devices from seeing each other, such as guest Wi-Fi and some mesh or office networks. There the announcement doesn't arrive, and players type the local address of the host instead, under **Join by Address**: the IPv4 address that `ipconfig` shows on the host PC, then a colon and the port Share shows, such as `192.168.1.20:17494`.

## VPN

A VPN like Tailscale or ZeroTier puts the PCs of friends on one private network over the internet. The war server treats players on the common VPN address ranges as local, so **LAN** is enough: no port to forward and no invite code. The announcement doesn't cross most VPNs, so players type the VPN address of the host and the port under **Join by Address**. A VPN is the safer choice for internet play, since only people invited to the VPN can reach the war at all.

## Internet

**Share**, then **Internet**. Share asks once more before opening, since the war then answers callers from anywhere who bring the code, and opening to the internet opens LAN too. The host then:

1. Forwards the TCP port shown in Share on the router to the host PC. Router menus call this port forwarding or virtual server.
2. Finds the public address of the connection, on the router status page or any "what is my IP" site, opened from the host PC.
3. Sends friends the address with the port, such as `203.0.113.7:17494`, and the invite code from Share.

Friends open **Join War** in Obsidian, type the address and the code under **Join by Address**, press **Find Wars** and join. The code stays with the joined war on that PC, so it is typed once.

Many home connections change the public address from time to time. When that happens, friends join again with the new address. Some providers put many homes behind one shared address (called CGNAT), and then no port forward reaches the host PC: a VPN works there.

## The invite code

A war open to the internet answers a caller from outside the network only with the code, which looks like `K7M2P-9XQ4T`. Without it, or with a wrong one, the war looks absent. The war gets a code the first time it opens to the internet and keeps it over restarts. **Copy Code** in Share copies it, and **New Code** replaces it, which shuts out everyone holding the old one until they join again with the new code.

## What is possible today

Sharing a war with friends works today, on a LAN, over a VPN or over the internet with the invite code. Players who join see the war screens of the host in view-only mode and play the war, and every mission they finish counts.

An open war that anyone can find and join is not supported yet. Posting the address and the code in public turns the code into a public password, and the war server has no defense past it:

- The war server takes the player ID each game reports without proof, so a stranger with the code could report missions under any name.
- The host can't kick or ban a single player. **New Code** is the only lock, and it locks out everyone.
- The connection is plain HTTP, readable on the way.
- The war server runs inside Obsidian on a player PC. It isn't made to run unattended on a rented server.

Tunnel services that forward traffic through a program on the host PC, such as playit.gg or ngrok, make every caller look local to the war server. The war then answers them without asking for the code, so use a port forward or a VPN instead.

A war holds up to 5000 players. Lobbies in the game stay at four players, as always.
