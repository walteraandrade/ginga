# Published on crates.io, under the same names

**Chosen.** `ginga` and `ginga-gpui` go to crates.io as 0.1.0. The repository is public and open source. Issue #2.

**Options.** Waltinho was shown three other pairs of names, all free on 2026-11-27 (`molejo`, `balanco`, `frevo`, each with `-gpui`), and kept `ginga`. For publishing he was shown: publish 0.1.0 now, prepare and leave publishing to him, or stay on git dependencies. He chose to publish now.

**Why.** Desafinado and anyone else can depend on a version instead of a revision. A published version cannot be deleted, only yanked, and the name stays with the account, so the names were his choice even though he had left them to an agent first.

**What follows from it.**

- Both crates use the root README. Its links are absolute GitHub URLs, because relative ones do not resolve on crates.io. A new link in the README is absolute too.
- CI runs `cargo package -p ginga`, which builds the packaged engine on its own. The adapter cannot be packaged with checks until the engine version it needs is on crates.io, so it is packaged with `--no-verify` before a release.
- To release: bump `version` in `[workspace.package]` and in `[workspace.dependencies]` together, then `cargo publish -p ginga`, then `cargo publish -p ginga-gpui`. Until 1.0, a change that breaks the API bumps the minor version.
- The first publish needs a crates.io token from the account that will own the names. Later releases can use crates.io's trusted publishing from GitHub Actions, which needs the crates to exist first.
