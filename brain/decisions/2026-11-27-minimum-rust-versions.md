# Minimum Rust versions: 1.87 for the engine, 1.95 for the adapter

**Chosen.** The workspace sets `rust-version = "1.87"` and the engine inherits it. The adapter overrides it with `rust-version = "1.95"`, with a comment that says why. CI checks each crate on its own version. Issue #3, pull request #17.

**Options.** The issue asked for one version in the workspace. One version for both would have to be 1.95, the adapter's.

**Why two.** The engine has no dependencies, and GPUI's requirement should not raise it.

- The engine needs 1.87: edition 2024 needs 1.85, and the tests and the bench use `u64::is_multiple_of`, which needs 1.87.
- `gpui-pre =0.3.12` needs 1.95. Each toolchain was tried in turn: 1.87 and 1.88 fail on dependencies that declare newer Rust, through 1.92 `gpui-pre-util` fails on `slice_as_array`, 1.93 and 1.94 fail on `cold_path`.

**To change it.** For one version, set the workspace to 1.95 and remove the adapter's line. Moving the GPUI pin moves the adapter's minimum: try the toolchains again. CI reads both versions from the manifests, so the manifests are the only place to change; the book's contributing page says them too.

Related: [[codebase/gpui-pin-and-the-orphan-rule]].
