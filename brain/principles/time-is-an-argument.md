# Time is an argument

**The rule.** Nothing in the engine reads a clock, starts a timer or spawns a task. Every call that depends on time takes `now: Duration` from its caller, and reading a motion (`sample`) takes `&self`.

**Why.** A motion that owns no clock is plain data: it can be tested by handing it times, written down as a [[glossary]] trace with no window, and driven by any framework's frame clock. A motion that read its own clock could not be checked without waiting, and two frames of one window could see two different times. Fixing the whole future when the target is set is also what lets a motion say when it settles (`settles_at`) before it moves.

**How to apply it in a review.**

- Look for `Instant::now`, `SystemTime`, `std::thread`, timers or async in `crates/ginga`. Any of them is a finding.
- A new method that depends on time takes `now` as an argument. A method that reads a value takes `&self`; one that takes `&mut self` only to read is a finding.
- A test of the change hands times in, and asserts on values at those times or on a `Trace`. A test that sleeps is a finding.
- In the adapter, the clock is read once per frame (`Clock`) and handed on. A second place that reads time is a finding.

Related: [[principles/closed-form-math]].
