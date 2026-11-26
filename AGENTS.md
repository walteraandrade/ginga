# AGENTS.md

ginga: motion for Rust UIs, as a pure function of time. The README says what it is and shows the whole API. These are the rules that keep it that way.

## Before you finish

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test                                  # seconds
cargo clippy -p ginga-gpui -- -D warnings   # only when crates/ginga-gpui changed; the first build takes minutes
```

## The engine (`crates/ginga`)

- No dependencies, and no word about any UI framework.
- Time is an argument. Nothing in the engine reads a clock, starts a timer or spawns a task.
- Reading a motion (`sample`) takes `&self` and changes nothing. A motion's whole future is fixed when its target is set.
- Running a motion never allocates. `tests/no_alloc.rs` fails otherwise. `Trace` allocates, because it is for tests and logs. `PresenceList` allocates when what it holds changes or something in it is leaving, and not otherwise.
- Math is closed-form: a value at a time is computed from where the motion started, never by stepping from the last frame. A skipped frame changes nothing.
- A motion says when it is settled, and a settled motion is exactly on its target. Whoever draws stops asking for frames then.

## The API

- Keep it small. Before adding a call, check that the ones in the README cannot do the job.
- A thing that leaves stays drawable: `Presence` and `PresenceList` keep its content until the way out is over. They never decide what leaving looks like; they give a phase and a progress.
- An invalid value does not build. Where a constructor can fail, it returns an error whose message says what to use instead.
- Every public item has a doc comment, and every doc example runs as a test.
- Names say what a thing does in plain words: `toward`, `jump_to`, `settled`.

## Checking motion

Do not check a motion by looking at a screen. Write it down:

```rust
println!("{}", Trace::of(&motion, start, Duration::from_millis(16)));
```

A test of a new behavior asserts on values at given times, or on a trace. Use a `Tween` with `Easing::Linear` when the numbers should be exact.

## The GPUI adapter (`crates/ginga-gpui`)

- It pins one GPUI: `gpui-pre =0.3.12`. An app must use the same one or the types will not match.
- It adds only what needs GPUI: the frame's clock, the frame request, reduced motion, and stand-ins for GPUI's own types. Motion logic goes in the engine.
- The orphan rule keeps it from implementing `ginga::Animatable` for GPUI's types. A new GPUI type gets an `Animate` impl with a stand-in.

## Rust

- Lints are in `[workspace.lints]`, clippy pedantic included. Fix the code; when a lint is wrong for one item, use `#[expect(lint, reason = "...")]`.
- `unsafe` is forbidden in the library crates.
- Each test asserts something about behavior.

## Issues and pull requests

- Work is tracked in GitHub issues: `gh issue list`. `ready-for-agent` can be done without asking; `ready-for-human` waits for a person's decision.
- An issue is one thin slice that can be shown working on its own, written in three sections: What to build, Acceptance criteria, Blocked by.
- Branch from `main` as `feat/`, `fix/`, `chore/`, `refactor/`, `docs/` or `test/` plus a short kebab-case name.
- Commit messages: short, imperative, conventional prefix (`feat:`, `fix:`, `refactor:`, ...).
- A pull request fills in `.github/PULL_REQUEST_TEMPLATE.md` and names the issue it closes. For a change in motion, its evidence is a `Trace` before and after.
