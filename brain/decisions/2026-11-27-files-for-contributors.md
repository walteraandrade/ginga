# Files for contributors

Chosen: the files GitHub's community profile and [opensource.guide](https://opensource.guide/) ask for, kept thin and pointing at what already exists (#48).

**Why:** the maintainer asked to get the repo ready for contributors, and GitHub's community profile marked a code of conduct, a contributing guide, a security policy and issue templates as missing. Thin pointers keep one source for each rule (`AGENTS.md`, the manifests, the CI workflow), so the files for contributors don't drift from it.

- `CONTRIBUTING.md` is for people: where to start, and links to `AGENTS.md` for the rules and checks, to this folder for the reasons, and to the manifests and CI workflow for Rust versions and system libraries. It repeats none of them; a rule changes in `AGENTS.md`, not there.
- `CODE_OF_CONDUCT.md` is the Contributor Covenant 2.1, unchanged except for how to reach the maintainer.
- `SECURITY.md` sends reports to GitHub's private vulnerability reporting, so no address is written in the repo. It only works while that setting is on: the maintainer turned it on in the repository's Security tab on 2026-11-27.
- Issue forms (`.github/ISSUE_TEMPLATE/`): a bug asks for a `Trace` and the values expected; a feature asks for the three sections an issue has (What to build, with why the calls in the README can't do it; Acceptance criteria, required; Blocked by). Blank issues are off.

Options passed over: a CODEOWNERS file (one maintainer, nothing to route), a SUPPORT file (the issue forms cover it), a CHANGELOG (nothing released yet; it came with the first release, 0.1.0, in #53).
