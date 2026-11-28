# Gliding to a new place

`.glides(id, transition)` makes an element travel to where layout puts it, instead of jumping there: a card in a wrapping row when one before it goes, a panel's content when a sidebar opens, a row when the one above it grows. It is drawn where it was in the frame of the move and heads for its new place, keeping its speed if the place changes again on the way.

```rust,ignore
div().flex().flex_wrap().children(self.cards.iter().map(|card| {
    div().w(px(120.)).h(px(72.)).child(card.title.clone()).glides(("card", card.id), Spring::SNAPPY)
}))
```

Only what is painted moves, and where the pointer finds it: layout is not touched, so nothing is laid out again while it glides and the elements around it take their places at once. A frame of a glide costs one sample of its motion and a frame request. A move is a change of its place in the window's layout: a scroll, an offset that a look or a drag paints it with, and a glide of an element around it are not moves, so an element that glides inside another that glides moves with it and glides only by how its place inside changes. A change of the window's size is not a move either. Its size is not moved: it takes its new size at once. Its first frame is in place, and under reduced motion it is in its new place at once, after its delay. The rows of GPUI's `list` and `uniform_list` are each laid out on their own: use `PresenceList::place_all` there. A cached view, a `deferred` element and an `anchored` one are laid out or drawn on their own too, so a glide inside one does not see it move: put the glide on the cached view, or around it. `.glides(..)` goes on any element, a look or another wrapper of ginga's included.
