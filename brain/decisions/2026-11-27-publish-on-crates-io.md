# Published on crates.io, under the same names

**Chosen.** `ginga` and `ginga-gpui` go to crates.io as 0.1.0. The repository is public and open source. Issue #2, pull request #53.

**Options.** For publishing, Waltinho was shown three choices: publish 0.1.0 now, prepare everything and leave publishing to him, or stay on git dependencies. He chose to publish now. The names are in [[decisions/2026-11-27-crate-names]].

**Why.** Desafinado, and anyone else, can depend on a version instead of a revision. A published version cannot be deleted, only yanked, and the name stays with the account, so the names were his choice even though he had left them to an agent first.

**What follows from it.**

- Both crates ship the root README. Its links are absolute GitHub URLs, because relative ones do not resolve on crates.io, so a new link in the README is absolute too.
- CI runs `cargo package --locked --workspace` in the adapter job. It builds both crates as crates.io gets them, the adapter against the packaged engine, before anything is published. `cargo package -p ginga-gpui` on its own fails until the engine version it needs is on crates.io.
- The packaged engine's README examples are the doc tests that `cargo test` runs. Packaging builds the crate but runs no tests.
- `CHANGELOG.md` gets an entry for each release, and `SECURITY.md` supports the latest published version.
- To release: write the CHANGELOG entry; bump `version` in `[workspace.package]` and in `[workspace.dependencies]` together; run `cargo package --locked --workspace`; then `cargo publish -p ginga`, then `cargo publish -p ginga-gpui`. Until 1.0, a change that breaks the API bumps the minor version.
- The first publish needs a crates.io token from the account that will own the names. Later releases can use crates.io's trusted publishing from GitHub Actions, which needs the crates to exist first.
