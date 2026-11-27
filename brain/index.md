# brain

ginga's memory, for agents and people: what earlier sessions decided and found, written down so a session that starts cold does not have to ask again. `AGENTS.md` is the short list of rules; this folder holds the reasons behind them and what a later task would need to know. GitHub issues stay the backlog and the plans.

## Read first

- [[principles]]: the ideas the rules in `AGENTS.md` come from, one per file.
- [[glossary]]: the words of ginga, and what each one means here.

## Principles

- [[principles/time-is-an-argument]]
- [[principles/closed-form-math]]
- [[principles/settled-is-exact]]
- [[principles/no-allocation-while-running]]
- [[principles/the-engine-knows-no-framework]]
- [[principles/an-invalid-value-does-not-build]]
- [[principles/a-small-api]]

## Decisions

- [[decisions/2026-11-27-license]]: MIT OR Apache-2.0 (#1).
- [[decisions/2026-11-27-crate-names]]: `ginga` and `ginga-gpui`, not published yet (#2).
- [[decisions/2026-11-27-minimum-rust-versions]]: 1.87 for the engine, 1.95 for the adapter (#3).
- [[decisions/2026-11-27-stagger-is-a-delay-on-the-transition]]: stagger is a delay on the transition (#8).
- [[decisions/2026-11-27-a-brain-folder-for-memory]]: this folder, and why not a memory server (#36).
- [[decisions/2026-11-27-colors-mix-in-linear-light-premultiplied]]: how two colors are mixed (#26).
- [[decisions/2026-11-27-keyframes-are-an-array-of-stops]]: keyframes keep their stops in an array (#30).

## Codebase

- [[codebase/gpui-pin-and-the-orphan-rule]]: one GPUI version, and why GPUI types move through stand-ins.
- [[codebase/testing-the-adapter]]: GPUI's test platform: no display, a clock the test moves, exact values.
- [[codebase/no-alloc-counts-one-thread]]: the allocation test counts the test's own thread only.
- [[codebase/bouncy-spring-bench-and-code-layout]]: why the bouncy one-channel spring benchmarks slow (#6).
- [[codebase/what-a-cached-view-saves]]: render, layout and paint, not the scene's trip to the GPU.
- [[codebase/recording-a-gallery-page]]: how a recording drives the gallery, and why it must never block the main thread.
- [[codebase/a-look-on-an-element]]: where an element's look motion lives, and how each value is applied.
- [[codebase/a-drag-on-an-element]]: how a dragged element hears the pointer anywhere in the window, and measures its speed.

## Measurements

They stay in `docs/measurements/`, with their scripts and patches:

- [The AI caret of Desafinado](../docs/measurements/2026-11-26-ai-caret/README.md): the engine costs nothing next to a frame.
- [The AI caret in an island](../docs/measurements/2026-11-26-ai-caret-island/README.md): an island keeps the heavy pane from rendering.
- [The island on its own](../docs/measurements/2026-11-27-island-alone/README.md): a synthetic heavy view, measured in a virtual display: the island more than halves the main thread's frame.

## How to write here

- One topic per file, about 50 lines at most; the glossary is the one exception. File names in lowercase with hyphens.
- Links are `[[folder/file]]`, so the folder opens as an Obsidian vault.
- Every file is reachable from this index: a new file gets a line here, or in [[principles]].
- A decision is named by its date, `decisions/YYYY-MM-DD-what-was-chosen.md`: what was chosen, the options, why, and the issue or pull request.
- An issue that asks for a choice to be written down (#26, #30, #31) adds its decision file here in its pull request.
- A principle says the rule, why it holds, and how to apply it in a review.
- A note that is wrong or replaced is deleted; git keeps its history.
- What a test, a lint or `AGENTS.md` already enforces is not repeated here.
