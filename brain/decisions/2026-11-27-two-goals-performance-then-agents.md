# Two goals: performance, then ease of use for AI agents

**Chosen.** ginga has two goals, in this order: 1. performance, 2. ease of use for AI agents. When they pull apart, performance wins. The README says so in "What it is for", with the inspirations each idea came from. Waltinho, 2026-11-27, in the thread that rewrote the README.

**Why this order.** Motion runs every frame, so its cost is paid all the time; an API that is easy for an agent but costs a frame is not worth it. Most rules serve both: closed-form math, time as an argument and a settled motion that is exact make motion cheap and also readable as text with a `Trace`.

**How to apply.** A new call or change says which goal it serves. One that makes the API easier to use but allocates while running, reads a clock or keeps asking for frames at rest does not go in. Between two calls that cost the same, pick the one an agent can check without a screen.

**Inspirations.** The README's table lists them and where each lives in the code: Motion (formerly Framer Motion), lilt, SwiftUI, Jetpack Compose, Neovide, gpui-motion, `gpui-base::motion`, gpui-ce-motion, react-spring, Svelte, GSAP, FLIP and Flutter's `Hero`. A new feature taken from another library adds a row there.

**Door.** Two-way.
