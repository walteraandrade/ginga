# Motion that redraws little

GPUI renders again the view that asks for an animation frame, and every view above it, on each frame of a motion. A view beside them is reused only when it is embedded with `.cached(style)`. So a motion is cheap when what moves is drawn by a small view of its own, the heavy views sit beside it and are cached, and the views above it are cheap.

`ginga_gpui::Island` is that small view: a state and a function that draws it.

```rust,ignore
let dot = cx.new(|_| {
    Island::new(Dot::default(), |dot, window, cx| {
        let x = dot.x.toward(dot.target, Spring::SNAPPY, window, cx);
        div().absolute().left(x).size_2().into_any_element()
    })
});

// In the parent: the heavy view is cached, the island sits beside it.
div()
    .relative()
    .child(self.document.clone().cached(StyleRefinement::default().size_full()))
    .child(dot.clone())
```

Measured on its own, with a heavy view of 2,400 boxes and a dot that never stops, a frame cost the main thread about 19 to 22 ms with the island and 48 to 50 ms without it, and the heavy view did not render again while the dot moved: [the measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-27-island-alone/README.md) (software rendering, no GPU). In an app it depends on what else asks for frames: in Desafinado a frame that skipped the heavy view cost about 40 % less, but only one frame in four skipped it: [that measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-26-ai-caret-island/README.md). With the requests tray's marks in an island too, most frames of a Review skip it, and the Review draws about 17 % less: [the next one](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-27-tray-marks/README.md). Two loops in two views draw on their own beats, so the window can draw more frames than either asks for.
