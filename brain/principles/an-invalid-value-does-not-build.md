# An invalid value does not build

**The rule.** A value the engine cannot run, such as a spring's bounce of `1.0` or more, is refused where it is built. Where a constructor can fail it returns a `Result`, and the error's message says what to use instead.

**Why.** A bad value that builds fails later, during a frame, far from the line that wrote it, or not at all: a spring with a bounce of `1.0` would never settle and the view would ask for frames forever. Refusing it at the constructor puts the failure where the mistake is, and a message that names the valid range turns it into a one-line fix. Presets (`Spring::SNAPPY` and the others) cover the common cases without a `Result`.

**How to apply it in a review.**

- A new parameter with a range that matters is checked in its constructor or builder, which returns a `Result` with an error type that has a doc comment and a `Display` message.
- The message says what is wrong and what to use, for example the range and its ends.
- A value that is checked only with `debug_assert!`, clamped silently, or left to panic in a frame is a finding.
- A test builds the invalid value and asserts the error.

Related: [[principles/a-small-api]].
