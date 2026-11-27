---
name: roundtable
description: Review a pull request with three reviewers in one round (Mat on standards, spec and tests; Poteto on architecture, quality, tests and performance; Addy on correctness, readability, architecture, security and performance) and post each finding as an inline PR comment. Use when asked for a roundtable, or to review a PR "with the roundtable".
---

# Roundtable

One round of review on one pull request. Three members sit at the table, and each is **exactly one agent** that carries its own brief:

| Member | Brief | Looks at |
|---|---|---|
| Mat | [members/mat.md](members/mat.md) | Standards, Spec, TDD |
| Poteto | [members/poteto.md](members/poteto.md) | Architecture, Code Quality, Tests, Performance, with tradeoff options |
| Addy | [members/addy.md](members/addy.md) | Correctness, Readability, Architecture, Security, Performance |

Members do not split their work into further agents. Each one walks all of its axes alone, so a round costs three agents, no more.

The members only read. Nothing in a round edits code, pushes, approves or requests changes. The deliverable is the comments on the PR.

## Arguments

- A PR number (`#21`, `21`) or a branch with an open PR. If none is given, use the open PR for the current branch; if there is none, ask.
- Optional per-member model, for a multi-model round: `mat=sonnet poteto=opus addy=fable`. A member without one inherits the session's model.
- Optional `--dry-run`: print the findings in the comment format and post nothing.

## Process

### 1. Pin the PR

Read the PR: title, body, base ref, head ref and head SHA, and the list of changed files. Get the diff against the merge-base:

```sh
git fetch origin <base> <head>
git diff origin/<base>...<head-sha>
git log --oneline origin/<base>..<head-sha>
```

Stop here if the PR is closed, the diff is empty, or the head SHA cannot be fetched. Record the head SHA: every comment is posted against it, and a member reviews that SHA and nothing in a working copy.

### 2. Gather the shared context

Every member gets the same packet. Collect it once:

- **Principles and standards**: `AGENTS.md` (the repo's rules, including the checks under "Before you finish") and `README.md` (the public API). `CLAUDE.md` only points to `AGENTS.md`.
- **Spec**: the issue the PR closes (`Closes #N` in the body, or `#N` in the commits). Read it with its three sections: What to build, Acceptance criteria, Blocked by. If there is none, say "no spec" in the packet.
- **Evidence the author gave**: the PR's Summary, Evidence (a `Trace` before and after for a change in motion) and Merge Danger sections.
- **Size**: lines changed and files touched. Poteto uses it to pick BIG or SMALL mode.
- **Commentable lines**: for each changed file, the line ranges on the RIGHT side of the diff (added or context lines in a hunk). Only these can take an inline comment.

### 3. Seat the members

Send **one message with three `Agent` calls**, `subagent_type: general-purpose`, one per member, so they run in parallel and do not see each other's work. Each prompt contains:

1. The member's brief: tell the agent to read `.claude/skills/roundtable/members/<member>.md` in full before anything else.
2. The shared packet from step 2, with the head SHA and the exact `git diff` command.
3. The output contract below, pasted as is.

Pass the member's `model` to the `Agent` call when the user gave one.

**Output contract** (paste into every member prompt):

> Return only a JSON array, nothing before or after it. Each element is one finding:
>
> ```json
> {"path": "crates/ginga/src/tween.rs", "line": 42, "severity": "Critical|Required|Optional|Nit|FYI", "finding": "…", "model": "…"}
> ```
>
> - `path` and `line` point at the RIGHT side of the diff at the head SHA, on a line inside a hunk. Use `"line": null` for a finding that has no single line (a missing test file, the PR's size, the spec).
> - `finding` is plain Markdown, at most about 120 words: what is wrong, where, why it matters, and the fix. Quote the code or the rule it breaks.
> - `model` is the exact model name your own environment says you are running on. Do not guess one.
> - Check each finding against the callers, guards and tests that reach the code, and drop it if they rule it out. Zero findings is a valid answer; return `[]`.
> - Do not edit files, commit, push or comment on GitHub yourself.

### 4. Post the findings

Do not merge, dedupe or rerank across members. Two members flagging the same line is a signal, and both comments stay. Inside one member, keep its order.

Turn every finding into a comment with exactly this body:

```
Roundtable Member: <Mat | Poteto | Addy>
Finding: <severity>: <finding>
Model: <model>
```

Post them all as **one review** on the head SHA with event `COMMENT`:

- A finding whose `line` is inside a hunk becomes an inline comment on `path` and `line`, side `RIGHT`.
- A finding with `"line": null`, or a line GitHub refuses, goes in the review's body instead, in the same three-line format, one block per finding, separated by a blank line. Prefix it with `` `path` `` when it has one.
- With the GitHub MCP tools: `pull_request_review_write` (method `create`, no event) to open a pending review, `add_comment_to_pending_review` once per inline finding, then `pull_request_review_write` (method `submit_pending`, event `COMMENT`, the body).
- Without them: one `gh api repos/<owner>/<repo>/pulls/<n>/reviews` call with `commit_id`, `event: COMMENT`, `body` and the `comments` array.

If no member found anything, post nothing on GitHub.

With `--dry-run`, print the comments grouped by member instead and stop.

### 5. Report

Reply with the review's link and one line per member: how many findings, and its worst one. Do not pick a winner across members.

## Why three seats

Each brief catches what the others let through. Mat asks whether the code follows this repo's rules, does what the issue asked, and is tested by behavior. Poteto asks whether the design is right, and makes every issue a choice with its cost. Addy asks whether the code is correct and safe, and labels how much each finding matters. Keeping them in separate agents stops one view from talking the others out of a finding.
