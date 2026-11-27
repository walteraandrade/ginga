# Files for contributors

Chosen: the files GitHub's community profile and [opensource.guide](https://opensource.guide/) ask for, kept thin and pointing at what already exists.

- `CONTRIBUTING.md` is for people; it points to `AGENTS.md` for the rules and to this folder for the reasons, and repeats only the steps (setup, checks, branches, commits). A rule changes in `AGENTS.md`, not there.
- `CODE_OF_CONDUCT.md` is the Contributor Covenant 2.1, unchanged except for how to reach the maintainer.
- `SECURITY.md` sends reports to GitHub's private vulnerability reporting, so no address is written in the repo. It only works while that setting is on (Settings, Code security).
- Issue forms (`.github/ISSUE_TEMPLATE/`): a bug asks for a `Trace` and the values expected; a feature asks for the three sections an issue has (What to build, Acceptance criteria, Blocked by) and why the calls in the README can't do it. Blank issues are off.

Options passed over: a CODEOWNERS file (one maintainer, nothing to route), a SUPPORT file (the issue forms cover it), a CHANGELOG (nothing released yet; it comes with the first release).
