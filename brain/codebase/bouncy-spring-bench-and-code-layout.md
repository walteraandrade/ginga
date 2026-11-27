# The bouncy spring benchmark and code layout

Issue #6 asked why a one-channel spring benchmarked slower than a four-channel one. Pull request #19 found two causes and explained the effect that is left; #6 was closed on that explanation on 2026-11-27.

**Fixed.**

- The bench's frames were 16.666 µs, not 16.666 ms, so it only measured the first millisecond of each motion.
- A frame that only draws computed every channel's velocity and dropped it. `sample` now asks for the distance alone (`Phase::left`) and skips the speed. A test asserts `Phase::left` equals `Phase::apply(..).0` exactly, so values are bit for bit what they were.
- The bench hides each motion behind `black_box(&mut motion)`, so the optimizer cannot fold one case differently from another.

**Explained, not fixed.** On x86 the bouncy (under-damped) one-channel spring still reports slower than four channels: 28 against 21 ns per frame. It is not the work. Valgrind counts 237 instructions per frame for one channel and 261 for four. The under-damped frame comes out at about 18 or about 28 ns depending on code layout, and small unrelated edits flip which case gets the slow layout. `#[inline(always)]` on `state`, `#[inline(never)]` on `phase`, multiplying by `sin / wd` instead of dividing, and aligning branches for the JCC erratum each moved the slow layout without removing it.

Sharing the distance formula between `Phase::left` and `Phase::apply` through a helper was tried in review and reverted: valgrind counted 252 instructions per four-channel frame against 165, though the bench's one-channel bouncy case got faster with it. The bench alone would have called it a win.

**So:** #6's measure is now instructions per frame. A few ns of difference between bench cases is not evidence by itself. Check instruction counts under valgrind before changing code for it.
