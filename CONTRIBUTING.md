# Contributing to ginga

Thank you for wanting to help. ginga is motion for Rust UIs, as a pure function of time; the [README](README.md) says what it is, and [the book](https://walteraandrade.github.io/ginga/) (its pages are in `book/`) shows its whole API. This file says how a change gets in.

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md). A security problem is not reported in a public issue: see [SECURITY.md](SECURITY.md).

## Where to start

- **A bug.** Open an issue with the bug form. The most useful report is a `Trace` of the motion that went wrong (see [Checking motion](#checking-motion)), with the times and values you expected.
- **An idea or a new call.** Open an issue with the feature form before writing code. ginga keeps its API small, so the first question is whether the calls in the book already do the job; a short talk in the issue saves a pull request that cannot be merged.
- **Something to pick up.** Issues labeled `ready-for-agent` are ready to work on, by a person or a coding agent. Issues labeled `ready-for-human` wait for a maintainer's decision; leave those until it is made. Say in the issue that you are taking it, so two people don't do the same work.
- **A typo or a broken link.** Send a pull request straight away.

## Read first

The rules are short and live in one place, [AGENTS.md](AGENTS.md). It is written for coding agents and applies to everyone. The reasons behind it are in [`brain/`](brain/index.md):

- [the principles](brain/principles.md): time is an argument, closed-form math, a settled motion is exact, no allocation while running, an engine that knows no framework, invalid values that do not build, a small API;
- [the glossary](brain/glossary.md): the words of ginga. Use them in issues, pull requests, docs and names; a new word goes there in the same change;
- [the decisions](brain/index.md#decisions) already made, with their reasons. A pull request that reopens one says so and why.

## Setting up

You need Rust through [rustup](https://rustup.rs), at least the `rust-version` each crate's manifest names; CI checks each crate on exactly that version.

```sh
git clone https://github.com/walteraandrade/ginga
cd ginga
cargo test        # the engine only, in seconds
```

The adapter, `crates/ginga-gpui`, takes minutes to build the first time. On Linux, GPUI links against a few system libraries: the `apt-get install` line in [the CI workflow](.github/workflows/ci.yml) lists them.

## Making a change

1. Branch from `main` and keep one change to one pull request: one thin slice that can be shown working on its own. Branch names, tests, docs and the gallery follow [AGENTS.md](AGENTS.md).
2. Check a motion by writing it down with a `Trace`, not by looking at a screen; [AGENTS.md](AGENTS.md#checking-motion) shows how.
3. When the change teaches something a later task would need (a decision, its reasons, a trap in the code), write it to `brain/`, following the rules in [`brain/index.md`](brain/index.md#how-to-write-here).
4. Before you push, run the checks in [AGENTS.md, "Before you finish"](AGENTS.md#before-you-finish). CI runs the same ones with `--locked`, on each crate's minimum Rust, and always runs the adapter's; a change to the engine can break the adapter, so run those too when in doubt.

## Commits and pull requests

- Commit messages are short and imperative, with a conventional prefix: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- A pull request fills in the [template](.github/PULL_REQUEST_TEMPLATE.md): a short summary, the evidence (a `Trace` before and after, for a change in motion), how far a mistake would reach, and the issue it closes.
- CI must be green. A maintainer reviews, and a pull request merges once every review comment is resolved.

## License

ginga is offered under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Unless you say otherwise, a contribution you submit for inclusion, as defined in the Apache-2.0 license, is offered under the same two, with no other terms.
