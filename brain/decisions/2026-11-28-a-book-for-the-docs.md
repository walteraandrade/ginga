# A book for the docs

**Chosen.** The docs are a book built with [mdBook](https://rust-lang.github.io/mdBook/) from `book/`, published to GitHub Pages at https://walteraandrade.github.io/ginga/ by `.github/workflows/book.yml` on each push to `main`. The README shrinks to one screen: what ginga is, its two goals in a paragraph, how to install it, one example, and links. Asked by Waltinho on 2026-11-28, who found the README too long (it had grown past 900 lines).

**Options.**

- Keep one long README. It was the whole API on one page, but too long to read, and crates.io showed all of it.
- docs.rs alone. It documents items, not how they work together, and has no place for the gallery, the costs or the inspirations.
- Another site generator (Zola, Docusaurus, VitePress). mdBook is the one Rust's own books use, is one binary, hides `# ` lines in examples as rustdoc does, and needs no Node.

**Why.** The book splits the README's sections into pages without rewriting them, and keeps its rule that examples run: `crates/ginga/src/lib.rs` includes every page under `cfg(doctest)`, so `cargo test` runs them as it ran the README's. Packaging leaves `book/` out; under `cfg(doctest)` the `include_str!` is never expanded, so `cargo package` still builds.

**What follows from it.**

- A new page goes in `book/src/SUMMARY.md` and gets a line in the `pages!` list in `lib.rs`.
- Links between pages are relative (`../gpui/look.md`); links in the README stay absolute, since crates.io ships it.
- The gallery's contact sheets are in `book/src/gallery/` as WebP, about 30 KB each.
- Pages must be set to deploy from GitHub Actions in the repository's settings, once.
