# Crate names: `ginga` and `ginga-gpui`, not published yet

**Chosen.** The engine is `ginga` and the GPUI adapter is `ginga-gpui`. Both crates stay `publish = false`. Issue #2, pull request #16, which refers to #2 and does not close it.

**Options.** Waltinho let an agent choose the names. The alternative was a new name if either was taken. Whether the repository goes public, and whether the crates go to crates.io, are left to Waltinho: both are hard to undo, and #2 is one of the issues kept for a person.

**Why.** Both names were free on crates.io on 2026-11-27, checked against a crate that exists so that an empty answer meant free. The adapter is named after the engine plus the framework, so another adapter would follow the same pattern.

**When publishing is decided.** The manifests are ready (repository, readme, keywords, a versioned path dependency). Publishing is `publish = true`, then `cargo publish -p ginga`, then `cargo publish -p ginga-gpui`. Write that decision in a new file here.
