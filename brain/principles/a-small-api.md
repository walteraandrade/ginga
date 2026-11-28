# A small API

**The rule.** The book (`book/`) shows the whole API. Before a call is added, check that the ones in the book cannot do the job. Names say what a thing does in plain words: `toward`, `jump_to`, `settled`.

**Why.** A motion library is easy to grow: every app asks for its one special case. Each call added is one more thing to document, test, keep closed-form and keep from allocating, and one more choice for whoever uses it. A small set of calls that compose (a transition, a delay on it, a presence) covers more than a large set of special ones, and stays readable in one short book. Plain names keep the book, the [[glossary]] and the code in the same words.

**How to apply it in a review.**

- A pull request that adds a public item says why the existing calls cannot do the job. Stagger, for example, became one `delay` on each transition, not a new stagger type ([[decisions/2026-11-27-stagger-is-a-delay-on-the-transition]]).
- A new public item appears in the book, on the page where it belongs, with an example that runs as a test, and its word is in the [[glossary]].
- A name taken from another library's jargon, or one that says how it is built and not what it does, is a finding.
- An option with one caller is a finding unless the issue asks for it.

Related: [[principles/the-engine-knows-no-framework]].
