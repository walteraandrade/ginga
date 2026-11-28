# Ways in and out

`present` draws a thing while it is there and keeps drawing it on its way out, with no field in the view: say each frame whether it is there, and how it comes and goes. It arrives from `arrive_from` and leaves to `leave_to`, each an offset and an opacity; wanted again on its way out, it turns around from where it is. While it leaves it takes no clicks, and the focus leaves it. Other content in its place swaps, crossing (`Swap::Sync`) or one after the other (`Swap::Wait`).

```rust,ignore
let ways = InOut::new(Spring::SNAPPY, Tween::new(ms(200), Easing::In));
present("toast", self.saved.then_some("Saved"), ways, |text| {
    div().px_4().py_2().rounded_lg().bg(green).child(*text)
})
.arrive_from(Look::new().offset(0., 24.).opacity(0.))
.leave_to(Look::new().opacity(0.))
```

Content that is there in the first frame arrives too. Without `arrive_from` it appears at once, and without `leave_to` it is gone at once. `.on_gone(|content, window, cx| ..)` is called with each content once it is gone, after the frame that finds it gone: to drop what the view kept for it, or to move the focus. An instant way out, or one that reduced motion makes instant, is over on the frame that starts it; content wanted again on its way out is not gone.
