# The requests tray's marks of Desafinado in an island

2026-11-27. The third measurement of ginga in [desafinado-rust-port](https://github.com/walteraandrade/desafinado-rust-port), after [the AI caret in an island](../2026-11-26-ai-caret-island/README.md). That one found that a frame which skipped the editor pane cost about 40 % less, but only one frame in four skipped it: the requests tray, drawn inside the pane, breathes for as long as a request runs.

## What changed

| Build | Where a working request's mark is drawn | Editor |
| --- | --- | --- |
| pane | in the tray, inside the pane, breathing there | `origin/main` |
| island | in a `ginga_gpui::Island` of its own beside the pane, over the spot the tray leaves empty for it | a patch on top of it |

In the island build the tray lays out an empty 8 px spot for each working mark and records where it is. The island reads those spots and draws the breathing marks there. The pane is cached and lays out after the island has drawn, so the marks follow their spots one frame late, and when a spot appears, moves or goes, one more frame is drawn for the island to catch up. The tray's own one-second tick, which redraws its times, still renders the pane.

Two batches were measured, on two machines:

- **First batch**, [`editor-with-tray-island.patch`](editor-with-tray-island.patch) on `9440ad6`. Here a spot change notified the pane, and the patch also took out main's span at the top of `Workspace::render`, so the two builds differed by that probe as well.
- **Second batch**, after review, [`editor-with-tray-island-2.patch`](editor-with-tray-island-2.patch) on `713e97f` (desafinado-rust-port#58 at `ae7cdb9`). A spot change notifies only the island, the probe is kept, and the spot bookkeeping has unit tests. The two builds differ only in where the marks are drawn.

## What was measured

The same harness as the two measurements before: a release build with `--features perf-probe`, Xvfb at 1600 × 1000, the scripted fake model, both binaries run under the same file name, five runs of each build. The scenario is the island measurement's fake Review ([`scenario.txt`](../2026-11-26-ai-caret/scenario.txt) on [`doc.md`](../2026-11-26-ai-caret/doc.md)), 3.2 s from the key that starts it, and a slow Refine ([`refine.txt`](refine.txt) on the same document), 4 s from the key that starts it.

### First batch

Runs alternating P I P I…, split with [`island_stats.py`](../2026-11-26-ai-caret-island/island_stats.py), unchanged.

| Run | Draw time, pane (ms) | Draw time, island (ms) | Frames with the pane, pane / island | Frames without the pane, pane / island | Median with the pane, island (ms) | Median without, island (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 172.3 | 132.2 | 43 / 16 | 14 / 39 | 3.40 | 1.79 |
| 2 | 163.5 | 130.6 | 42 / 15 | 14 / 48 | 3.08 | 1.60 |
| 3 | 156.3 | 136.6 | 41 / 16 | 15 / 47 | 2.76 | 1.78 |
| 4 | 161.4 | 146.7 | 43 / 16 | 14 / 48 | 3.40 | 1.79 |
| 5 | 159.6 | 155.3 | 41 / 16 | 15 / 47 | 3.88 | 1.86 |
| mean | 162.6 | 140.3 | | | | |

The pane build drew 56 or 57 frames a run, with medians of 2.88 to 3.21 ms for a frame with the pane and 1.49 to 1.81 ms without. Peak memory was 497 to 501 MB for both.

The draw time fell by 14 % on average, but the island build's runs climb, 132 to 155 ms, while the pane build's stay flat, so the gap per pair shrinks from 40 ms to 4 ms. The two ranges do not overlap, by 1 ms. That is drift within the batch, not a stable gain, and it is why the second batch was run in the order P I I P P I I P P I.

The Refine ran three times per build in this batch, with [`refine_stats.py`](refine_stats.py):

| Run | Frames, pane / island | `Workspace::render` calls, pane / island | Time in it (ms), pane / island | `EditorPane::render` calls, pane / island | Draw time (ms), pane / island |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 64 / 84 | 64 / 84 | 19.0 / 24.8 | 59 / 47 | 243.3 / 257.1 |
| 2 | 65 / 84 | 65 / 84 | 19.9 / 24.9 | 60 / 48 | 244.5 / 280.9 |
| 3 | 66 / 71 | 66 / 71 | 19.7 / 21.9 | 61 / 66 | 233.1 / 274.7 |

The island build drew more frames and spent more time drawing in every run, about 13 % more on average.

### Second batch

Runs in the order P I I P P I I P P I with [`runs.sh`](runs.sh), on another machine, split with [`tray_stats.py`](tray_stats.py), which is `island_stats.py` with the window as an argument and the workspace's renders added. Raw output: [`output-2.txt`](output-2.txt).

The Review, 3.2 s:

| Run | Draw time, pane / island (ms) | Frames, pane / island | Frames with the pane, pane / island | Median with the pane, pane / island (ms) | Median without, pane / island (ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 164.7 / 133.2 | 104 / 104 | 47 / 16 | 1.90 / 2.37 | 1.13 / 1.04 |
| 2 | 160.6 / 119.8 | 109 / 92 | 57 / 17 | 1.80 / 2.24 | 1.04 / 1.08 |
| 3 | 151.0 / 123.5 | 123 / 111 | 57 / 17 | 1.57 / 2.25 | 0.85 / 0.89 |
| 4 | 151.4 / 123.7 | 108 / 133 | 57 / 20 | 1.78 / 1.48 | 0.93 / 0.77 |
| 5 | 139.3 / 136.4 | 94 / 112 | 57 / 17 | 1.81 / 1.95 | 0.93 / 0.94 |
| mean | 153.4 / 127.3 | | | | |

The Refine, 4 s:

| Run | Draw time, pane / island (ms) | Frames, pane / island | Frames with the pane, pane / island | Frames without, pane / island |
| --- | ---: | ---: | ---: | ---: |
| 1 | 182.0 / 187.7 | 141 / 129 | 68 / 68 | 73 / 61 |
| 2 | 144.4 / 176.8 | 80 / 134 | 70 / 69 | 10 / 65 |
| 3 | 151.3 / 152.5 | 84 / 103 | 70 / 76 | 14 / 27 |
| 4 | 131.5 / 148.9 | 81 / 110 | 68 / 80 | 13 / 30 |
| 5 | 151.1 / 144.2 | 77 / 79 | 68 / 69 | 9 / 10 |
| mean | 152.1 / 162.0 | | | |

The workspace rendered once per frame in every run of both builds, about 0.1 ms a call on this machine (`ws ms` in the raw output).

## What it says

- **A Review skips the pane.** The pane is rendered in 16 to 20 frames of a Review instead of 41 to 57, in both batches and on both machines. That split does not depend on the machine's regime, and it is what issue #14 asked for. The tray's one-second tick accounts for three of the frames left, and the Suggestions the Review streams in for most of the rest.
- **A Review draws less.** In the second batch the draw time fell by 17 % on average and was lower in every pair, by 3 to 41 ms. The two builds' ranges, 139 to 165 ms and 120 to 136 ms, do not overlap. In the first batch the gain shrank run by run; read it as a direction, not a size.
- **A Refine draws more.** The pane is still rendered on nearly every frame of a Refine, because the Suggestion Card's "Refining" dots loop at 30 fps inside the pane. The marks' island loops at 30 fps too, on its own beat, so the window draws the frames of both loops: more frames, most of them cheap ones without the pane. The draw time rose by 13 % in the first batch and by 7 % in the second, higher in four pairs of five. Moving the dots off the pane would turn most of a Refine's frames into cheap ones; until then a Refine costs a little more.
- **The change is kept.** A Review runs for many seconds and gains in every pair; a Refine runs for a few and loses less than the Review gains. The loss is written down here and in the follow-up about the dots.
- A frame that skips the pane costs about as much as a frame of the pane build without it, in the same batch: about 1.7 ms in the first, 0.9 ms in the second. The island note found about 2.1 ms on its machine. The workspace is above both islands and is rendered on every frame.
- The island build does not draw more frames in a Review because a cheaper frame allows it; at 1 to 3 ms a frame the frame rate is set by requests, not by cost. The frame counts move run to run in both builds.

## What the island cost the editor

- The marks are painted above everything the pane draws over its text: a Suggestion Card, the bubble or the spelling menu that overlaps the tray's corner, in a narrow window, no longer covers them. The AI caret made the same trade.
- The marks follow their spots one frame late: for one frame a mark can sit where its row was, or over the pass mark of a request that has just answered.
- The tray's Cancel button was checked by hand under Xvfb on both release builds of the first batch: the row and the request go and no Suggestion arrives. The other buttons (Stop for a Review, Show for an answered request) run the same code as before; the island draws only the marks.

## The workspace

Each frame an island asks for renders every view above it, so the workspace is rendered on every frame of any island's motion. In the first batch's Refine it took about 0.3 ms a call to build its elements, some 8 % of the draw time, against about 0.35 ms that a Review frame saves by skipping the pane; its layout, prepaint and paint come on top and are not timed. On the second machine it took about 0.1 ms a call. Caching more of the workspace's children, or rendering less in it per frame, is the other half of making these frames cheap.

## Noise

The first batch ran at about 3 ms for a frame with the pane, the second at about 1.8 ms. Frame counts moved by up to a third between runs of one build in the second batch (77 to 141 in a Refine), mostly in the first run of each scenario. Read the split of frames by pane render first: it does not depend on the regime.
