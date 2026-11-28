"""Writes the book for language models, next to the built site.

Run after `mdbook build book`: python3 book/llms.py book/book

- `llms.txt`: what ginga is and a link to each page, with its first sentence (https://llmstxt.org).
- `llms-full.txt`: every page, in the order of `SUMMARY.md`, as one Markdown file.
- `<page>.md` beside each `<page>.html`: the page's Markdown, so the links in `llms.txt` stay Markdown.

In Rust examples, the lines rustdoc hides (`# Ok::<(), _>(())`) are shown, so code copied from
these files compiles as it is.
"""

import re
import sys
from pathlib import Path

SITE = "https://walteraandrade.github.io/ginga/"
BOOK = Path(__file__).parent
SRC = BOOK / "src"
OUT = Path(sys.argv[1] if len(sys.argv) > 1 else BOOK / "book")

ABOUT = (
    "ginga is an animation library for Rust user interfaces. It moves values (numbers, points, "
    "sizes, colors and your own types) with springs, tweens, keyframes and coasting, and keeps a "
    "thing drawable while it leaves. The engine, the `ginga` crate, has no dependencies and fits "
    "any Rust UI framework or game loop: time is an argument, the same times always give the same "
    "values, and running a motion never allocates. The `ginga-gpui` crate adapts it to GPUI, the "
    "UI framework of the Zed editor."
)
NOTES = [
    "Add it with `ginga = \"0.1\"`; for GPUI, `ginga-gpui = \"0.1\"`, which pins `gpui-pre =0.3.12`.",
    "Engine examples on these pages run as tests on every change. Examples marked `rust,ignore` are GPUI sketches; each GPUI recipe links to a gallery page that compiles.",
    "Check motion without a screen: `sample(now)` returns the value at any time, and `Trace::of(&motion, start, step)` writes it down as text.",
    "Until 1.0, a new minor version can change the API.",
]


def pages():
    """The pages `SUMMARY.md` links, in order, with the section each is under."""
    section = "Start here"
    for line in (SRC / "SUMMARY.md").read_text().splitlines():
        if line.startswith("# ") and line != "# Summary":
            section = line[2:].strip()
        link = re.search(r"\[([^\]]+)\]\(([^)]+\.md)\)", line)
        if link:
            yield section, link.group(1), link.group(2)


def show_hidden_lines(text):
    """Unhides the lines rustdoc hides (`# ` at the start) inside Rust code blocks."""
    out, rust = [], False
    for line in text.splitlines():
        if line.startswith("```"):
            rust = not rust and line[3:].split(",")[0].strip() in ("rust", "")
            if line.strip() == "```":
                rust = False
        elif rust and (line.startswith("# ") or line == "#"):
            line = line[2:]
        out.append(line)
    return "\n".join(out) + "\n"


# What each page answers, for `llms.txt`. A page not named here gets its first sentence.
ABOUT_PAGE = {
    "introduction.md": "What ginga is, its two crates, and its two goals: performance first, then ease of use for AI agents.",
    "getting-started.md": "The crates to add to Cargo.toml, and the GPUI version the adapter pins.",
    "when-to-use.md": "When ginga fits and when it does not: what it does, its non-goals, its limits in GPUI, and the alternatives.",
    "recipes.md": "A table from what you want to do to the call that does it, and complete examples that compile.",
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


def first_sentence(text):
    for block in text.split("\n\n"):
        block = block.strip()
        if block and not block.startswith(("#", "```", "|", "-", "!", "<")):
            sentence = re.split(r"(?<=\.)\s", " ".join(block.split()), maxsplit=1)[0]
            return re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", sentence).replace("**", "")
    return ""


def absolute(text, page):
    """Turns the page's relative links into links on the site, for a file read on its own."""
    base = SITE + str(Path(page).parent).strip(".") + ("/" if Path(page).parent != Path(".") else "")

    def link(match):
        label, target = match.group(1), match.group(2)
        if re.match(r"^[a-z]+:", target) or target.startswith("#"):
            return match.group(0)
        path, _, anchor = target.partition("#")
        path = re.sub(r"\.md$", ".html", path)  # an image keeps its path
        url = re.sub(r"/[^/]+/\.\./", "/", base + path)
        return f"[{label}]({url}{'#' + anchor if anchor else ''})"

    return re.sub(r"\[([^\]]*)\]\(([^)]+)\)", link, text)


def main():
    summary = list(pages())
    index = [f"# ginga\n\n> {ABOUT}\n"]
    index += [f"- {note}" for note in NOTES]
    full = [f"# ginga\n\n> {ABOUT}\n\nSource: {SITE}\n"]
    section = None
    for part, title, page in summary:
        text = show_hidden_lines((SRC / page).read_text())
        (OUT / page).write_text(text)
        if part != section:
            section = part
            index.append(f"\n## {part}\n")
        index.append(f"- [{title}]({SITE}{page}): {ABOUT_PAGE.get(page) or first_sentence(text)}")
        full.append(f"\n---\n\n<!-- {SITE}{page.replace('.md', '.html')} -->\n\n{absolute(text, page)}")
    (OUT / "llms.txt").write_text("\n".join(index) + "\n")
    (OUT / "llms-full.txt").write_text("\n".join(full))
    print(f"{len(summary)} pages written to llms.txt, llms-full.txt and .md files in {OUT}")


main()
