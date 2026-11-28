# An element to drag

`.draggable(&state, drag)` lets the pointer drag any styled element. While it is held it is exactly under the pointer, with no spring between them; let go, it keeps the pointer's speed and coasts. A `Drag` says how: `.along(Axis::Horizontal)`, `.within(min, max)` to spring back inside limits, `.snap_to(points)` to come to rest on the nearest one, and `.coast(Coast::SLOW)`. The view owns a `DragState`, `DragState::default()` or `DragState::at(offset)`: `offset()` is where the element was last drawn, `rest()` where it comes to rest, `is_held()` whether the pointer holds it, and `throw(velocity, &drag, cx)` lets it go as a pointer would, for a key or a button.

```rust,ignore
// The view owns `knob: DragState`. A slider's knob with five stops:
let stops = (0..5).map(|stop| point(px(93. * stop as f32), px(0.)));
div().size(px(28.)).draggable(&self.knob, Drag::new().along(Axis::Horizontal).snap_to(stops))
```

Like a look's, the offset moves what is painted and where the pointer finds the element, from wherever it is laid out. With snap points, the limits are left out: put the points inside them. Only the topmost element under the pointer is taken, and a move with the button up lets it go, as when a release was lost outside the window. Under reduced motion a thrown element is at once where it comes to rest.
