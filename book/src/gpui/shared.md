# One thing in two places

`.shared(key, transition)` says that elements in different places are one thing. When one of a key appears as another of it goes, it does not appear in its place: it travels there from where the other was, its place and its size changing together. A tab's mark slides to the next tab; a row opens into a card.

```rust,ignore
// In each tab: only the selected one draws the mark.
div().id(tab).children((tab == selected).then(|| {
    div().h(px(2.)).w_full().bg(accent).shared("tab-mark", Spring::SNAPPY)
}))
```

While it travels it is drawn above the rest, and its destination stays empty at its size, with its margins and its position, so nothing around it moves. A key names one thing in a window, so two tab bars need a key each. It comes only from an element of its key drawn in the frame before, in the same window; the frame it appears in measures where it goes and draws nothing, so a trip starts with one blank frame. Under reduced motion it is in its destination at once. Its content is laid out again at each size it passes, since GPUI cannot scale an element. Two elements of one key must have different ids around them, such as tabs that each have an id: to GPUI, two with the same ids around them are one element, which takes its new place at once.
