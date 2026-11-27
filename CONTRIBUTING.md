# Contributing to ginga

Thank you for wanting to help. ginga is motion for Rust UIs, as a pure function of time; the [README](README.md) shows what it is and its whole API. This file says how a change gets in.

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md). A security problem is not reported in a public issue: see [SECURITY.md](SECURITY.md).

## Where to start

- **A bug.** Open an issue with the bug form. The most useful report is a `Trace` of the motion that went wrong (see [Checking motion](#checking-motion)), with the times and values you expected.
- **An idea or a new call.** Open an issue with the feature form before writing code. ginga keeps its API small, so the first question is whether the calls in the README already do the job; a short talk in the issue saves a pull request that cannot be merged.
- **Something to pick up.** Issues labeled `ready-for-agent` are ready to work on, by a person or a coding agent. Issues labeled `ready-for-human` wait for a maintainer's decision; leave those until it is made. Say in the issue that you are taking it, so two people don't do the same work.
- **A typo or a broken link.** Send a pull request straight away.

## Read first

The rules are short and live in one place, [AGENTS.md](AGENTS.md). It is written for coding agents and applies to everyone. The reasons behind it are in [`brain/`](brain/index.md):

- [the principles](brain/principles.md): time is an argument, closed-form math, a settled motion is exact, no allocation while running, an engine that knows no framework, invalid values that do not build, a small API;
- [the glossary](brain/glossary.md): the words of ginga. Use them in issues, pull requests, docs and names; a new word goes there in the same change;
- [the decisions](brain/index.md#decisions) already made, with their reasons. A pull request that reopens one says so and why.

## Setting up

You need Rust through [rustup](https://rustup.rs). The engine builds with Rust 1.87 or newer and the GPUI adapter with Rust 1.95 or newer; CI checks each on exactly that version.

```sh
git clone https://github.com/walteraandrade/ginga
cd ginga
cargo test        # the engine only, in seconds
```

The adapter, `crates/ginga-gpui`, takes minutes to build the first time. On Linux, GPUI links against a few system libraries:

```sh
sudo apt-get install -y libxkbcommon-x11-dev libwayland-dev libfontconfig-dev libfreetype-dev
```

## Making a change

1. Branch from `main` as `feat/`, `fix/`, `chore/`, `refactor/`, `docs/` or `test/` plus a short kebab-case name, for example `fix/spring-overshoot`.
2. Keep one change to one pull request: one thin slice that can be shown working on its own.
3. Write a test that asserts on behavior: values at given times, or a trace.
4. Document every public item. Every doc example runs as a test.
5. A new feature of the adapter adds a page to the gallery, `crates/ginga-gpui/examples/gallery`.
6. When the change teaches something a later task would need (a decision, its reasons, a trap in the code), write it to `brain/`, following the rules in [`brain/index.md`](brain/index.md#how-to-write-here).

Before you push, run what CI runs:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo clippy -p ginga-gpui --all-targets -- -D warnings   # when crates/ginga-gpui changed
cargo test -p ginga-gpui                                  # the same; no display needed
```

Lints include clippy's pedantic set. Fix the code; when a lint is wrong for one item, use `#[expect(lint, reason = "...")]`.

## Checking motion

Don't check a motion by looking at a screen. Write it down:

```rust
println!("{}", Trace::of(&motion, start, Duration::from_millis(16)));
```

Use a `Tween` with `Easing::Linear` when the numbers should be exact. A pull request that changes motion shows a `Trace` before and after.

## Commits and pull requests

- Commit messages are short and imperative, with a conventional prefix: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- A pull request fills in the [template](.github/PULL_REQUEST_TEMPLATE.md): a short summary, the evidence, how far a mistake would reach, and the issue it closes.
- CI must be green. A maintainer reviews, and a pull request merges once every review comment is resolved.

## License

ginga is offered under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Unless you say otherwise, a contribution you submit for inclusion, as defined in the Apache-2.0 license, is offered under the same two, with no other terms.
