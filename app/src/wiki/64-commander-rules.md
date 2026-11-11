---
title: War rules
group: Wars
summary: How fronts, enemy pressure, defenses and home world attacks move, and every setting with the default.
wiki: commander
---

## Fronts and regions

A Vanilla war has three fronts, one per enemy. Each front holds points from 0 to **Points per Front**, and ten regions share them equally: region 1 touches Super Earth and region 10 touches the enemy home world. A season starts every active front at **Start Points**, in region 2 by default.

A planet completed in the game adds **Planet Value** times the difficulty and the mission success to the front of that planet. While no event runs on a front, **Enemy Pressure** takes points away every war hour.

## Defenses

With the **War Director** on, each war hour has a **Defense Chance** of starting a defense on a random active front, in the current region of that front. One defense runs at a time. Each defense mission won adds **Defense Mission Value** times the difficulty, and reaching **Defense Goal** before **Defense Duration** runs out wins the defense and adds **Defense Reward** to the front. A failed defense costs the front one region.

A front that falls to 0 points starts a defense of Super Earth. Holding it puts the front back at the start of region 2, and losing it loses the season.

## Home world attacks

A front that reaches the last point starts an attack on the enemy home world. Each attack mission won adds **Attack Mission Value** times the difficulty, and reaching **Attack Goal** before **Attack Duration** runs out defeats that enemy. A failed attack drops the front back to 80% of **Points per Front**, in region 9.

## Settings

The three presets of **Create War** set these numbers. **Settings** on the war screen changes any of them while the war runs, and a change applies from then on. With the War Director off, only the Game Master starts defenses, while fronts still move with the missions and the enemy pressure.

| Setting | Standard | Short Campaign | Hard |
|---|---|---|---|
| Points per Front | 100,000 | 30,000 | 100,000 |
| Start Points | 15,000 | 3,000 | 15,000 |
| Enemy Pressure, per hour | 600 | 250 | 1,200 |
| Planet Value | 250 | 250 | 200 |
| War Director | On | On | On |
| Defense Chance, per hour | 8% | 12% | 16% |
| Defense Duration | 4 h | 2 h | 3 h |
| Defense Goal | 3,000 | 1,500 | 4,500 |
| Defense Mission Value | 150 | 150 | 120 |
| Defense Reward | 3,000 | 1,500 | 2,000 |
| Attack Duration | 6 h | 3 h | 4 h |
| Attack Goal | 5,000 | 2,500 | 8,000 |
| Attack Mission Value | 150 | 150 | 120 |
