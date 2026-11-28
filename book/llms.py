"""Writes the book for language models, next to the built site, and checks what it wrote.

Run after `mdbook build book`: python3 book/llms.py book/book

- `llms.txt`: what ginga is and a link to each page, with what the page answers (https://llmstxt.org).
- `llms-full.txt`: every page, in the order of `SUMMARY.md`, as one Markdown file.
- `<page>.md` beside each `<page>.html`: the page's Markdown, so the links in `llms.txt` stay Markdown.

What it writes is the page as mdBook shows it: `{{#include ..}}` is expanded, and in Rust
examples the lines rustdoc hides (`# ` at the start) are shown. It exits with an error when a page
of `SUMMARY.md` has no line in `ABOUT_PAGE`, or when a link it wrote does not reach the built site.
"""

import re
import sys
import tomllib
from pathlib import Path

SITE = "https://walteraandrade.github.io/ginga/"
BOOK = Path(__file__).parent
SRC = BOOK / "src"
ROOT = BOOK.parent
OUT = Path(sys.argv[1] if len(sys.argv) > 1 else BOOK / "book")

# What each page answers, for `llms.txt`. Every page of `SUMMARY.md` has a line; the run fails otherwise.
ABOUT_PAGE = {
    "introduction.md": "What ginga is, its two crates, and its two goals: performance first, then ease of use for AI agents.",
    "getting-started.md": "The crates to add to Cargo.toml, and the GPUI version the adapter pins.",
    "when-to-use.md": "When ginga fits and when it does not: what it does, its non-goals, its limits in GPUI, and the alternatives.",
    "recipes.md": "A table from what you want to do to the call that does it, and whole programs that compile.",
    "engine/motion.md": "`Motion`: a value toward a target with a spring or a tween; delays and staggers; keyframes and loops; coasting after a throw; derived values with `Map`; sequences.",
    "engine/presence.md": "`Presence`, `PresenceList` and `Slot`: things that come and go and are drawn while they leave; lists that reorder.",
    "engine/trace.md": "`Trace`: a motion written down as a table of times and values, for tests and logs.",
    "gpui/view.md": "`ginga_gpui::Motion` in a GPUI view: the frame's clock, frame requests and reduced motion.",
    "gpui/look.md": "`.look(id, look, transition)`: declare how a GPUI element looks (offset, opacity, size, radius, colors) and let it get there.",
    "gpui/look-moment.md": "`.hover(..)`, `.pressed(..)`, `.focused(..)`: a look while the pointer is over an element, while it is pressed, or while it has focus.",
    "gpui/ways-in-and-out.md": "`present(..)`: an element that arrives and leaves, drawn on its way out, with no field in the view.",
    "gpui/glide.md": "`.glides(id, transition)`: an element travels to where layout puts it instead of jumping.",
    "gpui/opened.md": "`.opened(id, progress)`: an element opens to its own height, as an accordion.",
    "gpui/shared.md": "`.shared(key, transition)`: one thing in two places; it flies from one to the other.",
    "gpui/variants.md": "`Variant`, `.leads(..)`, `.follows(..)`: a parent's state that its children follow one after another.",
    "gpui/drag.md": "`.draggable(&state, drag)`: drag an element, throw it, keep it inside limits or on snap points.",
    "gpui/loop.md": "`ginga_gpui::Keyframes`: a loop in a view, and a cap on how often it draws.",
    "gpui/island.md": "`Island`: keep a motion in a small view so the heavy views beside it are not rendered again.",
    "gallery.md": "The gallery app: a page for each feature, and how to record one.",
    "cost.md": "What a frame of motion costs, measured, and why the engine never allocates while it runs.",
    "coming-from.md": "From Motion (Framer Motion), SwiftUI and Jetpack Compose to ginga: a table and recipes.",
    "not-yet.md": "What ginga does not do yet, and the limits that come from GPUI.",
    "inspirations.md": "The libraries ginga learned from, the idea taken from each, and where it lives.",
    "contributing.md": "How to contribute, the checks to run, the minimum Rust versions and the license.",
}


def pages():
    """The pages `SUMMARY.md` links, in order, with the section each is under."""
    section = "Start here"
    for line in (SRC / "SUMMARY.md").read_text().splitlines():
        if line.startswith("# ") and line != "# Summary":
            section = line[2:].strip()
        link = re.search(r"\[([^\]]+)\]\(([^)]+\.md)\)", line)
        if link:
            yield section, link.group(1), link.group(2)


def as_shown(page):
    """The page as mdBook shows it: includes expanded, the lines rustdoc hides shown."""
    text = re.sub(
        r"\{\{#include ([^}\s]+)\}\}",
        lambda m: (SRC / page).parent.joinpath(m.group(1)).read_text().rstrip("\n"),
        (SRC / page).read_text(),
    )
    out, rust = [], False
    for line in text.splitlines():
        if line.startswith("```"):
            opening = not rust and line.strip() != "```"
            rust = opening and line[3:].split(",")[0].strip() == "rust"
        elif rust and (line.startswith("# ") or line == "#"):
            line = line[2:]
        out.append(line)
    return "\n".join(out) + "\n"


def plain(text):
    """Markdown links and bold as plain text."""
    return re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text).replace("**", "")


def about():
    """What ginga is: the introduction's first paragraph, and the versions from the manifests."""
    intro = (SRC / "introduction.md").read_text().split("\n\n")
    first = next(block for block in intro if block.strip() and not block.startswith("#"))
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]
    major, minor, _ = workspace["package"]["version"].split(".")
    gpui = workspace["dependencies"]["gpui"]
    version = f"{major}.{minor}"
    notes = [
        f'Add it with `ginga = "{version}"`; for GPUI, `ginga-gpui = "{version}"`, which pins `{gpui["package"]} {gpui["version"]}`: an app must use the same GPUI.',
        "Engine examples on these pages run as tests on every change; each engine recipe is a whole `main.rs`. GPUI recipes are whole gallery views that CI compiles.",
        "Check motion without a screen: `sample(now)` returns the value at any time, and `Trace::of(&motion, start, step)` writes it down as text.",
        "Until 1.0, a new minor version can change the API.",
        f"Every page below in one file: {SITE}llms-full.txt",
    ]
    sentences = re.split(r"(?<=[.:])\s+", " ".join(plain(first).split()))
    return " ".join(s for s in sentences if not s.endswith(":")), notes  # without the sentence that leads into a list


def absolute(text, page):
    """Turns the page's relative links into links on the site, for a file read on its own."""
    folder = str(Path(page).parent)
    base = SITE + ("" if folder == "." else folder + "/")

    def link(match):
        label, target = match.group(1), match.group(2)
        if re.match(r"^[a-z]+:", target) or target.startswith("#"):
            return match.group(0)
        path, _, anchor = target.partition("#")
        url = base + re.sub(r"\.md$", ".html", path)  # an image keeps its path
        while re.search(r"/[^/.]+/\.\./", url):
            url = re.sub(r"/[^/.]+/\.\./", "/", url, count=1)
        return f"[{label}]({url}{'#' + anchor if anchor else ''})"

    return re.sub(r"\[([^\]]*)\]\(([^)]+)\)", link, text)


def broken_links(files):
    """Links to the site in `files` that do not reach a built file, or an id on a page."""
    broken = []
    for name in files:
        for url in re.findall(re.escape(SITE) + r"[^)\s>]*", (OUT / name).read_text()):
            path, _, anchor = url[len(SITE):].partition("#")
            target = OUT / (path or "index.html")
            if not target.is_file():
                broken.append(f"{name}: {url} (no such file)")
            elif anchor and f'id="{anchor}"' not in target.read_text():
                broken.append(f"{name}: {url} (no such anchor)")
    return broken


def main():
    assert absolute("[x](../gpui/look.md#y)", "engine/motion.md") == f"[x]({SITE}gpui/look.html#y)"
    assert absolute("[x](#y) [z](https://a.b/c.md)", "cost.md") == "[x](#y) [z](https://a.b/c.md)"

    summary = list(pages())
    named = {page for _, _, page in summary}
    if named != set(ABOUT_PAGE):
        sys.exit(f"ABOUT_PAGE and SUMMARY.md differ: missing {sorted(named - set(ABOUT_PAGE))}, extra {sorted(set(ABOUT_PAGE) - named)}")

    pitch, notes = about()
    index = [f"# ginga\n\n> {pitch}\n"] + [f"- {note}" for note in notes]
    full = [f"# ginga\n\n> {pitch}\n\nSource: {SITE}\n"]
    section = None
    for part, title, page in summary:
        text = as_shown(page)
        if not text.startswith("# "):
            sys.exit(f"{page} does not start with its title")
        (OUT / page).write_text(text)
        if part != section:
            section = part
            index.append(f"\n## {part}\n")
        index.append(f"- [{title}]({SITE}{page}): {ABOUT_PAGE[page]}")
        full.append(f"\n---\n\n<!-- {SITE}{page.replace('.md', '.html')} -->\n\n{absolute(text, page)}")
    (OUT / "llms.txt").write_text("\n".join(index) + "\n")
    (OUT / "llms-full.txt").write_text("\n".join(full))

    broken = broken_links(["llms.txt", "llms-full.txt"])
    for line in broken:
        print(line)
    if broken:
        sys.exit(f"{len(broken)} broken links in llms.txt and llms-full.txt")
    print(f"{len(summary)} pages written to llms.txt, llms-full.txt and .md files in {OUT}")


main()
