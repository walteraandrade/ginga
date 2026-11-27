# The allocation test counts one thread

`crates/ginga/tests/no_alloc.rs` installs a counting global allocator and asserts that thousands of frames of reading and changing course allocate nothing ([[principles/no-allocation-while-running]]).

**The count must be thread-local.** The test harness allocates on its own threads while a test runs, and other tests in the same binary run in parallel. A single global counter therefore sees allocations that are not the motion's. It was rare on a laptop and failed at once on a CI runner (`left: 5, right: 0`). Pull request #17 made the counter thread-local, so it counts only the test's own thread; #21 carries the same patch, because it adds a second test to the file.

**It still catches a real allocation.** With a `vec![1u8]` added to the loop, the thread-local version fails with `left: 10000`.

**When adding to the file:** put the new behavior inside the counted section, not in setup before it. Something that allocates once when it is built (a `PresenceList` filled by its first `sync`) is built before the count starts.
