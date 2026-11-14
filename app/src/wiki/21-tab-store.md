---
title: Store
group: Know your tabs
summary: Mods ready to install in one click, and how to get a mod into the Store.
---

The [Store](app:store) lists mods that install with one click, newest first. The **Obsidian Kit** sits in a row at the top: Obsidian API, Obsidian Patches and Obsidian Trainer, the mods Obsidian is built around.

## Finding a mod

The row over the cards filters by [scope](wiki:scopes), sorts the list and keeps only installed mods. The search box matches names and descriptions.

Each card shows the picture, the name, a short description and the tags:

| Tag | Meaning |
|---|---|
| Silver square | **Obsidian Verified**: the mod is made and kept up by the Obsidian team, or by someone who earned their trust |
| Colored square | The [scope](wiki:scopes) of the mod |
| Early access | A version under 1.0: still in development, and later versions can change a lot |

A green corner marks an installed mod, and a yellow corner a mod with a newer version in the Store.

## The page of a mod

A click on a card opens the page of the mod: the description, the versions with what each one changed, the options, and the mods it requires or that require it. The panels at the right open with a click on the title.

**Install** downloads the newest version from GitHub, with every mod it requires, into the library. The mod arrives switched off: switch it on in the [Mods](app:mods) tab and deploy. Once installed, the button beside **Installed** removes the mod from the library.

## Adding a mod to the Store

The Store reads a public list, the [Obsidian Store Index](https://github.com/Alomare/ObsidianStoreIndex). A mod gets in when the repository is on that list:

1. Publish the mod in a public GitHub repository, with `obsidian.json` at the root (see [The manifest](wiki:manifest)).
2. Make releases tagged `v` and the version, such as `v1.0.0`, with the mod zip attached. The Store installs the newest stable release.
3. Optionally, add `thumbnail.jpg` (16:9) at the root of the repository and an icon named by `image` in the manifest. The README becomes the description on the page of the mod.
4. Open an issue or a pull request on the Store Index that adds the repository to `mods` in `mods.json`.

The index is rebuilt every six hours from the releases, so a new release shows in the Store within a few hours. Obsidian Verified is given by the Store Index to the GitHub accounts it trusts: the tag follows the account that owns the repository, never the authors a manifest names.
