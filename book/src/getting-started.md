# Getting started

Early work: until 1.0, a new minor version can change the API. Two crates, on crates.io:

| Crate | What it is | Depends on |
| --- | --- | --- |
| [`ginga`](https://github.com/walteraandrade/ginga/tree/main/crates/ginga) | The engine: springs, tweens, keyframes, coasting, colors, looks, presences and slots, derived values, sequences, trails and the trace. | nothing |
| [`ginga-gpui`](https://github.com/walteraandrade/ginga/tree/main/crates/ginga-gpui) | GPUI views: the frame's clock and frame requests, reduced motion, a look declared on an element, variants, ways in and out, shared elements, dragging, loops and islands. | `gpui-pre =0.3.12`, the GPUI that `gpui-kit` 0.9.1 builds on |

```toml
[dependencies]
ginga = "0.1"
ginga-gpui = "0.1" # for GPUI views
```

To follow `main` instead, depend on them from GitHub, both from the same revision:

```toml
[dependencies]
ginga = { git = "https://github.com/walteraandrade/ginga" }
ginga-gpui = { git = "https://github.com/walteraandrade/ginga" }
```

`ginga-gpui` pins one GPUI, `gpui-pre =0.3.12`: an app must use the same one, or the types do not match. It also re-exports the engine as `ginga_gpui::ginga`.
