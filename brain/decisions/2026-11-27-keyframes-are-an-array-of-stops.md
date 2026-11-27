# Keyframes are an array of stops

**Chosen.** `Keyframes<T, const N: usize>` keeps its stops in a `[Stop<T>; N]`, checked once in `Keyframes::new`. A round, a repeat count, mirroring, a pause and a start time are plain fields beside it. Issue #30.

**Options.**

- An array of `N` stops, the length in the type.
- A `Vec<Stop<T>>`.
- A borrowed slice, `&'a [Stop<T>]`.
- A chain of motions, one per stop, played one after another.

**Why.**

- An array keeps keyframes `Copy` and makes building and reading them allocate nothing ([[principles/no-allocation-while-running]]). A `Vec` would allocate when they are built, and building keyframes in a render is normal.
- A slice would put a lifetime on every view that holds keyframes, for no gain over an array, which the compiler sizes from the literal.
- A chain of motions would step from stop to stop, so a skipped frame would have to replay every stop it jumped over. With stops in one array, the value at a time is a search for the stop after it and one easing ([[principles/closed-form-math]]).
- The cost is a const generic in the type: `Keyframes<f32, 3>`. Inference fills it in from the array.

**What follows from it.**

- A value is found by a linear search over the stops. Keyframes have a handful of stops; a binary search would only pay past dozens.
- Stops at one time are allowed and are a jump. A first stop not at zero or stops out of order do not build (`KeyframesError`), and the message says what to do instead.
- A loop forever is never settled, and `settles_at` is `None` ([[principles/settled-is-exact]]). `ginga_gpui::Keyframes::at_most_every` is how a slow loop costs less than the display's rate.
