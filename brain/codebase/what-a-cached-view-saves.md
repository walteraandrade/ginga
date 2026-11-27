# What a cached view saves

GPUI 0.3.12 (`gpui-pre`), `src/view.rs`. A view embedded with `.cached(style)` and not notified since the last frame:

- does not render: its `Render::render` is not called, so nothing in it is built again;
- is not laid out: it is laid out from `style`, which is why caching needs a definite size;
- is not prepainted or painted: `prepaint_view` calls `window.reuse_prepaint(..)` and `paint_view` calls `window.reuse_paint(..)`, which copy the ranges it recorded last frame into this one.

What is left each frame is not the view's own work: the scene, with every primitive it holds, still goes to the GPU, so its encoding, submission and drawing grow with what the cached view draws. With a software driver the pixels are drawn on the CPU too, mostly on the driver's own threads.

The view above an island is rendered on each of the island's frames (it is on the path to the view that asked for the frame). Keep it cheap; the cached view beside it is skipped.

Seen in [the island measurement](../../docs/measurements/2026-11-27-island-alone/README.md): the heavy view rendered 0 times beside the island, and about 20 ms of main-thread time per frame was left with lavapipe.

Caching is skipped while GPUI's inspector is picking an element, and a view whose opacity changed is redrawn on the next frame.

Order within a frame, seen in the gallery's island page under Xvfb: the island renders *before* the cached view beside it, and the cached view renders twice while the window opens. A count a view shows of another view's renders is therefore one frame old; the page counts renders since Play instead.
