"""Checks the built book for broken links: every relative link, #anchor and image must resolve.

Run after `mdbook build book`: python3 book/check-links.py book/book
"""

import os
import re
import sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else "book/book")
pages = {p: p.read_text() for p in root.rglob("*.html")}
ids = {p: set(re.findall(r'id="([^"]+)"', text)) for p, text in pages.items()}
broken = []
for page, text in pages.items():
    if page.name in ("404.html", "print.html", "toc.html"):
        continue  # mdBook's own pages, with links rooted at the site's URL
    for attr, link in re.findall(r'(href|src)="([^"]+)"', text):
        if re.match(r"^[a-z]+:", link):
            continue  # absolute: http, https, mailto
        path, _, anchor = link.partition("#")
        target = Path(os.path.normpath(page.parent / path)) if path else page
        if target.is_dir():
            target = target / "index.html"
        if not target.exists():
            broken.append(f"{page.relative_to(root)}: {link} (no such file)")
        elif anchor and target in ids and anchor not in ids[target]:
            broken.append(f"{page.relative_to(root)}: {link} (no such anchor)")
for line in broken:
    print(line)
print(f"{len(pages)} pages, {len(broken)} broken links")
sys.exit(1 if broken else 0)
