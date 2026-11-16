---
title: Account
group: Know your tabs
summary: The GitHub sign-in, the access Obsidian holds, issue reports and sync between PCs.
---

The [Account](app:account) tab, the person icon at the right end of the top bar, connects Obsidian to a GitHub account. Everything else in Obsidian works without one. A check mark on the icon means an account is signed in.

## GitHub

**Sign in with GitHub** shows a code and opens the GitHub sign-in page. Type the code there to finish. Obsidian never sees the GitHub password.

The **Access** list shows what GitHub allows Obsidian to do, and **Allow** asks GitHub for one more kind of access:

| Access | What it is for |
|---|---|
| Public repository access | Reporting issues, replying to reports, starring mods and submitting a mod to the Store |
| Gist access | Sync between PCs |
| Private repository access | Early access mods from private repositories the account can read. GitHub grants this as read and write access to every repository the account can reach, so Obsidian asks for it only when allowed here |

**Sign out** makes Obsidian forget the account on this PC. The access stays granted on GitHub until **Revoke on GitHub** removes it there.

## Reports

Issues sent with **Report an issue** on a mod page in the [Store](wiki:tab-store) are listed here, with the mod, the state (open, closed or fixed) and the date. A number marks new replies. A click on a report shows the conversation, where a reply can be written and sent. **Open on GitHub** opens the issue in the browser, and **Remove from list** takes it off this list without closing it on GitHub.

## Sync

Sync keeps the profiles and the appearance settings the same on several PCs. It stores them in a secret gist on the GitHub account, which needs gist access.

- **Upload** saves the profiles and appearance settings of this PC to the gist.
- **Fetch** reads them on another PC. Profiles with the same name are replaced, and mods the profiles use that are missing from the library install from their GitHub repositories. Mods without a repository have to be added by hand, and Obsidian names them.
- **Open the gist** shows the stored file on GitHub.
