# A look for a moment

An element can take another look while the pointer is over it, while it is pressed, and while it has the keyboard focus. Each one names only what changes, and is laid over the element's own look with `Look::or`. Pressed wins over hover, and hover wins over focused. When the moment ends, the element goes back the same way, from where it is: a pointer that crosses quickly turns the motion around without a jump. The element's own handlers still run.

```rust,ignore
div()
    .id("save")
    .track_focus(&self.focus)
    .on_click(cx.listener(|this, _, _, cx| this.save(cx)))
    // A value moves in a moment only when the element's own look names it too:
    // without the transparent border, the ring would snap on and off.
    .look("save", Look::new().size(120., 40.).background(fill).border_color(Color::TRANSPARENT), Spring::SNAPPY)
    .hover(Look::new().size(128., 44.))
    .pressed(Look::new().offset(0., 2.))
    .focused(&self.focus, Look::new().border_color(ring))
```

`Look::or` lays any two looks over each other:

```rust
use ginga::Look;

let rest = Look::new().size(120.0, 40.0).opacity(0.8);
assert_eq!(Look::new().opacity(1.0).or(rest), Look::new().size(120.0, 40.0).opacity(1.0));
```
