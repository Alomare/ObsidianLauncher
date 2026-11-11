---
title: Sharing a mod
group: Making mods
summary: Packing a release, versions, pictures and translations.
---

## Packing

Zip the mod folder so that `obsidian.json` is at the top of the archive, or inside one folder at the top. Players drop the `.zip` onto the mod list, and that is the whole install.

Leave out files the mod doesn't use. Keep paths in `obsidian.json` relative to the mod folder, with forward slashes.

## Versions

Use a version number like `1.4.2`:

- raise the last number for fixes;
- raise the middle number for new features and options;
- raise the first number when the mod changes in a way players would want to know before updating.

A player who imports a higher version keeps option values and keys, except for options that changed type. Keep option and key ids the same between releases.

> [!WARNING]
> Never change the `id` of a released mod. Obsidian would see a different mod, and players would end up with both.

## Updates

With a GitHub repository named in `obsidian.json`, players update from the app:

```json
"source": "https://github.com/author/mod"
```

For each new version:

1. Raise `version` in `obsidian.json`.
2. Make a GitHub release whose tag is that version, such as `v1.4.2`.
3. Attach the `.zip` to the release. If the release holds several, put the mod id in the name of the right one, such as `author.mod-name-1.4.2.zip`.

Obsidian reads the latest release of a public repository and skips drafts and prereleases, which leaves those free for testing. It refuses a `.zip` with a different id or a version that isn't higher.

## Pictures

Add a square `image.png` (or JPG or WebP, up to 4 MB) and name it in `obsidian.json` with `"image": "image.png"`. 512 by 512 pixels is plenty.

## Translations

- Put the name, description and option labels in every language available, under `name`, `description` and `label`.
- Put the text the mod shows in game in `locales/<language>.json`.
- English is always the fallback, so a missing translation shows English instead of a blank.

## Before a release {#before-you-release}

1. Import the `.zip` itself into Obsidian, not the folder, to test what players get.
2. Deploy, play a mission, and read the log of the mod.
3. For a session mod, test with a friend who doesn't run it: the mod should switch off ([Writing session mods](wiki:session-mods)).
4. Write in the description what the mod changes and whether other players notice.
