# The AI caret of Desafinado in an island

2026-11-26. The second measurement of ginga in [desafinado-rust-port](https://github.com/walteraandrade/desafinado-rust-port). The [first one](../2026-11-26-ai-caret/README.md) found that the engine costs nothing next to a frame: a motion costs what GPUI renders for it. This one tries to render less.

## What changed

GPUI renders again the view that asks for an animation frame and every view above it. A view beside them is reused from the last frame only when it is embedded with `.cached(style)` and nothing notified it.

| Build | Where the AI caret is drawn | The editor pane |
| --- | --- | --- |
| pane | inside the pane, as before | rendered on every frame of the caret's motion |
| island | in a `ginga_gpui::Island` beside the pane, in the workspace | embedded with `.cached(..)` |

The change to the editor is [`editor-with-island.patch`](editor-with-island.patch), on top of `origin/main`.

## What was measured

The same scenario, harness and window as the first measurement: a fake Review, 3.2 s from the key that starts it, five runs of each build, alternating, both binaries under the same file name. [`island_stats.py`](island_stats.py) also splits the frames by whether the pane was rendered in them (the app times `EditorPane::render` as the span `pane.render`).

| Run | Draw time, pane (ms) | Draw time, island (ms) | Island: frames with the pane | median (ms) | Island: frames without the pane | median (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 200.9 | 185.7 | 41 | 3.84 | 15 | 2.07 |
| 2 | 214.8 | 163.0 | 42 | 3.53 | 14 | 2.38 |
| 3 | 204.5 | 168.4 | 43 | 3.41 | 14 | 1.55 |
| 4 | 193.7 | 195.8 | 44 | 3.81 | 14 | 2.50 |
| 5 | 170.9 | 168.4 | 42 | 3.35 | 15 | 2.07 |
| mean | 197.0 | 176.3 | | | | |

The pane build drew 57 frames in every run, all of them with the pane, with medians from 2.97 to 3.92 ms.

## What it says

- A frame that skips the pane is cheaper in every run: about 2.1 ms against about 3.6 ms, some 40 % less.
- Only one frame in four skips it. The pane is still notified by motion of its own: the requests tray, drawn inside the pane, breathes for as long as the Review runs.
- The frames that skip the pane still cost 2 ms. The workspace is above the island, so it is rendered on every frame, and nothing else in it is cached.
- The whole window's draw time fell by 10 % on average, lower in four runs of five. With the spread between runs (171 to 215 ms for one build) that is not a result to lean on. The split inside each run is.

So the rule holds and the gain here is small until two more things move: the other looping motion leaves the pane, and the workspace gets cheaper to render or more of its children are cached.

## What the island cost the editor

- The caret is now painted after the pane, so it is above the pane's Cards, bubble and tray where they overlap. Inside the pane it was below them.
- A cached pane is only rendered again when something notifies it. One place in the workspace changed the pane without notifying it and was fixed. A change that forgets to notify will now show a stale pane, where before every frame hid the mistake.
- Not checked beyond one recorded Review: every other flow of the app with a cached pane.

## Noise

This batch ran in the slow regime described in the first measurement (3 to 4 ms a frame; the first measurement's table was taken at 1.2 ms). Peak memory was 479 MB for the pane build and 497 MB for the island build, the same unexplained 15 to 18 MB step seen there between two runs of one binary. Neither is read as a result.
