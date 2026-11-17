"""The Linux ZIP for Nexus Mods, which refuses .AppImage uploads: the AppImage and the .deb of a release, and a
README.txt with how to install each.

    python tools/linux_zip.py <version> <folder with the two files> [<out folder>]

Writes ObsidianLauncher-<version>-linux.zip into the out folder (the files' folder when left out) and prints its path.
The AppImage is stored with the run permission, so unzip tools on Linux keep it runnable. Used by
.github/workflows/nexus.yml, and by hand for a release that is already out.
"""

import sys
import time
import zipfile
from pathlib import Path

README = """Obsidian Launcher {v} for Linux (beta)

Obsidian.Launcher_{v}_amd64.AppImage
Runs on any distribution and on SteamOS (in Desktop Mode). Allow it to run as a program (chmod +x, or the
Permissions tab of the file properties) and start it. On Ubuntu 24.04 and later, the AppImage needs FUSE 2 first:
sudo apt install libfuse2t64

Obsidian.Launcher_{v}_amd64.deb
For Ubuntu, Linux Mint, Pop!_OS and Debian:
sudo apt install ./Obsidian.Launcher_{v}_amd64.deb
New versions may ask for the system password.

Both install new versions from inside the app.
More: https://github.com/Alomare/ObsidianLauncher#linux-beta
"""


def entry(name, mode):
    info = zipfile.ZipInfo(name, date_time=time.localtime()[:6])
    info.create_system = 3  # Unix, so the mode below is read
    info.external_attr = (0o100000 | mode) << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    return info


def main():
    if len(sys.argv) not in (3, 4):
        sys.exit(__doc__)
    version, src = sys.argv[1], Path(sys.argv[2])
    out_dir = Path(sys.argv[3]) if len(sys.argv) == 4 else src
    appimage = src / f"Obsidian.Launcher_{version}_amd64.AppImage"
    deb = src / f"Obsidian.Launcher_{version}_amd64.deb"
    for f in (appimage, deb):
        if not f.is_file() or f.stat().st_size == 0:
            sys.exit(f"{f.name} is missing from {src}")
    out = out_dir / f"ObsidianLauncher-{version}-linux.zip"
    with zipfile.ZipFile(out, "w") as z:
        z.writestr(entry("README.txt", 0o644), README.format(v=version))
        z.writestr(entry(appimage.name, 0o755), appimage.read_bytes())
        z.writestr(entry(deb.name, 0o644), deb.read_bytes())
    print(out)


if __name__ == "__main__":
    main()
