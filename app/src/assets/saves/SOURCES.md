# Sources of the save editor pictures

Decide the rights before a release: no license is recorded for any of these.

HELLDIVERS and the game's art belong to Arrowhead Game Studios and Sony Interactive Entertainment. Every picture here is game art as published on the [Helldivers Wiki](https://helldivers.wiki.gg). They were downloaded for the Heroic UI research (`Helldivers 1/Mods/HeroicUI/internal/wiki/img`, where `_meta.json` holds the address of each file) and renamed for the save editor concepts (`Handoff/saves-concepts/src/img`, `map.json` maps each item to a file).

| Files | What | Wiki files |
| --- | --- | --- |
| `rank-1.png` to `rank-50.png` | Rank insignia, 144 px (the 36 px wiki files at four times the size) | `01_-_Private.png` to `50_-_Admiral_of_Super_Earth.png` |
| `weapon_*.png` | Weapon silhouettes, 192 x 84 | `<weapon name>_silhouette.png`, such as `LAS-5_Scythe_silhouette.png` |
| `stratagem_*.png` | Stratagem icons, 96 x 96 | `HD1_Stratagem_<name>_mk1.png` |
| `perk_*.png` | Perk pictures | mostly `<perk name>_perksmall.png`, the grenades from `HD_<kind>'Nade.PNG` |

Each file is named after the catalog item (`core/assets/save-catalog.json`), so a new item with no file here shows the fallback glyph.
