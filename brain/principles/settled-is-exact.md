# Settled is exact

**The rule.** A motion says when it is settled, and a settled motion is exactly on its target: not within a hair of it. Whoever draws stops asking for frames then.

**Why.** A view asks for the next frame while something on it moves. If "settled" were only "close enough", a value could stop a little short and be drawn there for good, or a view could never stop asking for frames and burn power while nothing visible changes. Snapping to the target once every channel is within its precision makes the end exact and the stop clean. The [[glossary]] keeps *settled* (a value) apart from *present* (something shown).

**How to apply it in a review.**

- A new kind of motion reports settled from the same closed-form state as its value, and once settled returns the target itself.
- `settles_at` counts everything that delays the end, such as a delay before the start.
- A motion that never settles (a loop that repeats forever) says so in its docs and tests, and says what a view should do about frames.
- In the adapter, a frame is requested only while a motion is not settled. A request after settling is a finding.
- A test asserts the last row of a trace equals the target exactly, not approximately.

Related: [[principles/closed-form-math]].
