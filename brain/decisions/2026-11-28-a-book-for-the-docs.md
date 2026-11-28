# A book for the docs

**Chosen.** The docs are a book built with [mdBook](https://rust-lang.github.io/mdBook/) from `book/`, published to GitHub Pages at https://walteraandrade.github.io/ginga/ by `.github/workflows/book.yml` on each push to `main`. The README shrinks to one screen: what ginga is, its two goals in a paragraph, how to install it, one example, and links. Asked by Waltinho on 2026-11-28, who found the README too long (it had grown past 900 lines).

**Options.**

- Keep one long README. It was the whole API on one page, but too long to read, and crates.io showed all of it.
- docs.rs alone. It documents items, not how they work together, and has no place for the gallery, the costs or the inspirations.
- Another site generator (Zola, Docusaurus, VitePress). mdBook is the one Rust's own books use, is one binary, hides `# ` lines in examples as rustdoc does, and needs no Node.

**Why.** The book splits the README's sections into pages without rewriting them, and keeps its rule that examples run: `book/` is also a crate, `ginga-book` (`publish = false`, a default member), whose `build.rs` reads `src/SUMMARY.md` and gives each linked page to rustdoc as an item's doc comment, so `cargo test` runs them as it ran the README's.

The pages were first included from `crates/ginga/src/lib.rs` (#97's review). That broke `cargo test` on the packaged engine, which does not ship `book/`, and put GPUI's page names in the engine. A list of pages written by hand could also drift from `SUMMARY.md` and drop a page's tests without a failure; generating it from `SUMMARY.md` cannot.

**What follows from it.**

- A new page needs only its line in `book/src/SUMMARY.md`: its examples run from then on.
- `mdbook build` does not fail on a broken link or anchor; `book/check-links.py` checks the built site, in CI and before finishing.
- Links between pages are relative (`../gpui/look.md`); links in the README stay absolute, since crates.io ships it.
- The gallery's contact sheets are in `book/src/gallery/` as WebP, about 30 KB each.
- For coding agents, the book opens by saying what ginga is, for which framework, and what it does, and has a page on when to use it and one of recipes by intent, each engine recipe a whole `main.rs` (an explicit `fn main()`) run as a test, and each GPUI recipe a gallery view included with `{{#include}}`, so CI compiles what an agent copies. `book/llms.py` writes `llms.txt`, `llms-full.txt` and a `.md` beside each page into the built site ([llmstxt.org](https://llmstxt.org)). It reads the pitch from the introduction and the versions from `Cargo.toml`; a new page needs a line in its `ABOUT_PAGE`, or the run fails, and so does a link in what it wrote that does not reach the built site. Asked by Waltinho on 2026-11-28: an agent looking for a Rust animation library should see at once what ginga is for.
- Pages must be set to deploy from GitHub Actions in the repository's settings, once.
- A trap the review found in a recipe: when a `PresenceList` item at rest is let go, the items after it take their new place at once; they do not travel. To make the room close, call `place_all` on every frame with a running total of each row's height times its `progress`, as the list recipe does.
