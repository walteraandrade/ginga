# A brain/ folder for memory

**Chosen.** This folder: plain Markdown in the repository, in the layout of poteto's [noodle `brain/`](https://github.com/poteto/noodle/tree/main/brain), cut down to principles, a glossary, decisions and codebase notes. GitHub issues stay the backlog and the plans, so there is no `todos.md` and no `plans/`. Issue #36.

**Options.**

- [ai-memory](https://github.com/akitaonrails/ai-memory), by Fabio Akita: a server with lifecycle hooks that records sessions and keeps a git-backed wiki, shared across agents and machines.
- Cline's Memory Bank (`projectbrief.md`, `activeContext.md`, `progress.md`, ...).
- Only `AGENTS.md`.

**Why.** ai-memory is too much for one library: it needs a binary and a server running, and the memory would live outside the repository and its review. Its wiki is plain Markdown, so it can be added later on top of this folder. The Memory Bank's progress and current-work files repeat what issues and pull requests already hold. `AGENTS.md` alone would grow past a short list of rules, and every session would read all of it.

**What follows from it.** Memory changes go through pull requests and review like code. The roundtable review's Poteto member (#35) reads [[principles]].
