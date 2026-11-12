"""Publishes the draft release of a tag as the latest one, after tools/verify_release.py has passed and the
GitHub build has been walked. Publishing is what the updater sees, and it starts the Nexus Mods upload.

Usage: python tools/publish_release.py v1.2.0
"""
import sys

import github_api

REPO = "Alomare/ObsidianLauncher"


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    tag = sys.argv[1]
    rel = github_api.release(REPO, tag)
    if not rel["draft"]:
        raise SystemExit(f"{tag} is already published")
    done = github_api.publish(REPO, rel)
    print(f"{tag}: draft {done['draft']}, {done['html_url']}")
    sys.exit(0 if not done["draft"] else 1)


if __name__ == "__main__":
    main()
