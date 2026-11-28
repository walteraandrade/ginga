# Principles

The ideas the rules in `AGENTS.md` come from. Each file says the rule, why it holds, and how to apply it in a review. A reviewer reads these and the files they link.

## Time

- [[principles/time-is-an-argument]]: the engine never reads a clock; whoever draws hands it the time.
- [[principles/closed-form-math]]: a value at a time comes from where the motion started, never from the last frame.
- [[principles/settled-is-exact]]: a settled motion is exactly on its target, and then nothing asks for frames.

## Cost

- [[principles/no-allocation-while-running]]: a motion that runs never touches the heap.

## Shape of the crates

- [[principles/the-engine-knows-no-framework]]: the engine has no dependencies; the adapter adds only what needs GPUI.
- [[principles/an-invalid-value-does-not-build]]: a bad value is an error at the constructor, with a message that says what to use.
- [[principles/a-small-api]]: a new call has to show that the ones in the book cannot do the job.
