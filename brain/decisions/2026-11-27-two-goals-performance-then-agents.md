# Two goals: performance, then ease of use for AI agents

**Chosen.** ginga has two goals, in this order: 1. performance, 2. ease of use for AI agents. When they pull apart, performance wins. The book says so in "What it is for" (its introduction), with the inspirations each idea came from on a page of their own; the README says it in one paragraph. Chosen by Waltinho on 2026-11-27; pull request #51.

**Options.** The reverse order (agents first), the two with equal weight, or performance alone. Waltinho named both goals and their order.

**Why this order.** Motion runs every frame, so its cost is paid all the time; an API that is easy for an agent but costs a frame is not worth it. Most rules serve both: closed-form math, time as an argument and a settled motion that is exact make motion cheap and also readable as text with a `Trace`.

**How to apply.** A new call or change should be able to say which goal it serves. One that makes the API easier to use but allocates while running, reads a clock or keeps asking for frames at rest does not go in. Between two calls that cost the same, pick the one an agent can check without a screen.

**Inspirations.** The table in `book/src/inspirations.md` lists them and where each lives in the code. A new feature taken from another library should add a row there.

**Door.** Two-way.
