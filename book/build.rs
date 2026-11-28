//! Writes one item for each page that `src/SUMMARY.md` links, with the page as its doc comment.

use std::{env, fmt::Write as _, fs, path::Path};

fn main() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let summary = src.join("SUMMARY.md");
    println!("cargo::rerun-if-changed={}", summary.display());
    let summary = fs::read_to_string(&summary).expect("book/src/SUMMARY.md is readable");

    let mut items = String::new();
    for (n, page) in summary
        .split("](")
        .skip(1)
        .filter_map(|rest| rest.split_once(')'))
        .map(|(link, _)| link)
        .enumerate()
    {
        let path = src.join(page);
        assert!(
            path.is_file(),
            "SUMMARY.md links {page}, which is not a file in book/src"
        );
        println!("cargo::rerun-if-changed={}", path.display());
        writeln!(
            items,
            "#[doc = include_str!({:?})]\npub struct Page{n};",
            path.display().to_string()
        )
        .expect("writing to a String cannot fail");
    }

    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("pages.rs");
    fs::write(out, items).expect("OUT_DIR is writable");
}
