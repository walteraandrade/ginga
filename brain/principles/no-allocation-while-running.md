# No allocation while running

**The rule.** Reading a motion, and changing its course, never touches the heap. `Trace` allocates, because it is for tests and logs. `PresenceList` allocates only when what it holds changes or something in it is leaving.

**Why.** Motion runs on every frame, often many motions in one frame, for as long as something moves. An allocation per frame per motion is the kind of cost that hides in a profile as "the allocator" and grows with the screen. A motion that is a fixed-size value, with a fixed number of channels, can be stored inline in a view and copied freely. It also keeps the engine usable where allocation is not wanted at all.

**How to apply it in a review.**

- A new type a motion holds has a size known at compile time: no `Vec`, `Box`, `String` or `HashMap` in a motion or a transition. Stops of a keyframe motion, for example, need a fixed-size store or a borrowed slice.
- `tests/no_alloc.rs` covers the new behavior: the change runs through thousands of frames and changes of course inside the counted section. A new feature without a line there is a finding.
- Something that allocates on purpose says when in its doc comment, as `PresenceList` does.

Related: [[codebase/no-alloc-counts-one-thread]].
