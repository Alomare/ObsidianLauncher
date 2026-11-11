# Contributing to Obsidian

Thanks for helping. Obsidian is a small project, so a short note before a large change saves both of us time: open an issue first and describe what you want to do.

## Branches

Work happens on `dev`. Open pull requests against `dev`. `main` only moves when a release is published.

## Building and testing

You need Rust (stable, MSVC toolchain on Windows), the Visual Studio Build Tools and Node.js.

1. Build the LuaJIT 2.0 compiler once: `tools\build-luajit.cmd`
2. Run the core tests: `cargo test`
3. Run the loader tests: `tools\test-loader.cmd`
4. Run the UI tests: `npm ci`, then `npm test`, in `app`
5. Run the app: `tools\dev.cmd`

`python tools/affected_tests.py` prints the checks that cover what changed since the last push (or since a ref: `python tools/affected_tests.py main`), and `--run` runs them. Pushes to `dev` run the tests of the parts that changed.

On Linux you also need the [system packages Tauri lists](https://v2.tauri.app/start/prerequisites/#linux). Use `sh tools/build-luajit.sh` for step 1 and `target/luajit/luajit loader/tests/run.lua` for step 3.

Some tests read your own game files and are skipped unless you point them at the game with `OBSIDIAN_GAME_DIR`. Game files are never committed.

## The Obsidian Kit

Obsidian API, Obsidian Trainer and Obsidian Patches each have their own repository, under the MIT License. `bundled/` holds a copy of each one for the installer and the tests. It isn't committed: run `python tools/starter_pack.py` once after cloning, and it fetches each mod's latest release, as the release workflow does. So a change to a mod reaches Obsidian's next build once that mod is released. To work on a mod and Obsidian together, clone its repository into `../Bundled` and use `--local`, which takes what the clones have committed.

## Changes that need care

- Anything in the loader that decides when session mods run (spec section 6) needs a test for every case it touches, and a note in the pull request on how it was tested in game.
- Obsidian never writes a vanilla game file. A change that does will not be merged.
- Mods that change the game for other players must stay `session` scope. Pull requests that weaken the guardrails will not be merged.

## Bug reports

Include the output of `obsidian status`, the file `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs\loader.log` if one exists, and the mods you had enabled.

## License

By contributing, you agree that your contribution is licensed under the [Apache License 2.0](LICENSE), the same as the rest of Obsidian. The Obsidian Kit repositories are under the MIT License.
