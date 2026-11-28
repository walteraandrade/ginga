//! The book's examples, run as tests.
//!
//! `build.rs` reads `src/SUMMARY.md` and writes one item for each page it links, with the page as
//! its doc comment, so `cargo test` runs every example of every page in the book, and a new page
//! needs nothing more than its line in `SUMMARY.md`. This crate is not published: the engine's
//! package ships only its README, whose examples `crates/ginga` runs.

#[cfg(doctest)]
include!(concat!(env!("OUT_DIR"), "/pages.rs"));
