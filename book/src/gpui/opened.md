# Opening to its own height

`.opened(id, progress)` gives an element `progress` of its own height, as an accordion opens: at `0.0` it takes no room, at `1.0` it is the element as it is, and part of the way open the rest is cut off and takes no clicks. Its own height is measured as it is laid out, at the width it has, so wrapped text opens to the height it wraps to. The progress comes from whatever moves it, such as a presence:

```rust,ignore
let answer = self.answer.frame(window, cx).map(|shown| {
    div().p_2().child(*shown.content).opened("answer", shown.progress)
});
div().child(question).children(answer)
```

It is held in a box that takes its place in the parent: its margins, its width, its share of a flex row and its position are the box's, so it is laid out the same at every progress, and its margins stay around it, closed too. A progress past `0.0..=1.0`, such as a bouncy spring's, is held to it. It keeps the height it measured last under its id: the first frame drawn part of the way open, and a frame where the content's height changes part of the way open, draw the height before and ask for the next frame. A height given as a share of the parent's, such as `h_full`, has no height of its own to open to. Part of the way open, only what is above or below the box is cut, so a shadow at its sides stays; closed, nothing of it is drawn, so it takes no clicks, no focus and no keys, and it asks for no frame.

What it costs is layout and rendering: while it opens, its parent is laid out again on each frame, and cached views in it or moved by it render again on each frame, since a cached view is reused only at the same place, size and cut. A cached view beside its parent does not. Fully open and still, it asks for nothing. [The measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-28-accordion/README.md).

`.height_glides(transition)` makes the height it opens to glide when the content's own height changes, as a toast whose text grows, instead of jumping. It heads for the new height from where it is and keeps its speed when the content changes again on the way. The frame that finds the change draws the height before, and what is above or below the box is cut while it glides. Under reduced motion the new height is taken at once, after the transition's delay. For something that is always open, give it a progress of `1.0`:

```rust,ignore
div().p_2().child(self.message.clone()).opened("toast", 1.).height_glides(Spring::SNAPPY)
```
