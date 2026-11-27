# The requests tray's marks of Desafinado in an island

2026-11-27. The third measurement of ginga in [desafinado-rust-port](https://github.com/walteraandrade/desafinado-rust-port), after [the AI caret in an island](../2026-11-26-ai-caret-island/README.md). That one found that a frame which skipped the editor pane cost about 40 % less, but only one frame in four skipped it: the requests tray, drawn inside the pane, breathes for as long as a request runs.

## What changed

| Build | Where a working request's mark is drawn | Editor |
| --- | --- | --- |
| pane | in the tray, inside the pane, breathing there | `origin/main` at `9440ad6` |
| island | in a `ginga_gpui::Island` of its own beside the pane, over the spot the tray leaves empty for it | [`editor-with-tray-island.patch`](editor-with-tray-island.patch) on top of it |

In the island build the tray lays out an empty 8 px spot for each working mark and records where it is. The island reads those spots and draws the breathing marks there. When a spot appears, moves or goes, the pane asks for one more frame, so the island catches up on the next one. The tray's own one-second tick, which redraws its times, still renders the pane.

## What was measured

The same harness as the two measurements before: a release build with `--features perf-probe`, Xvfb at 1600 × 1000, the scripted fake model, both binaries run under the same file name, five runs of each build, alternating. The scenario is the island measurement's fake Review ([`scenario.txt`](../2026-11-26-ai-caret/scenario.txt) on [`doc.md`](../2026-11-26-ai-caret/doc.md)), 3.2 s from the key that starts it. [`island_stats.py`](../2026-11-26-ai-caret-island/island_stats.py) is the island measurement's script, unchanged: it splits the frames by whether `EditorPane::render` (the span `pane.render`) ran in them.

| Run | Draw time, pane (ms) | Draw time, island (ms) | Frames with the pane, pane / island | Frames without the pane, pane / island | Median with the pane, island (ms) | Median without, island (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 172.3 | 132.2 | 43 / 16 | 14 / 39 | 3.40 | 1.79 |
| 2 | 163.5 | 130.6 | 42 / 15 | 14 / 48 | 3.08 | 1.60 |
| 3 | 156.3 | 136.6 | 41 / 16 | 15 / 47 | 2.76 | 1.78 |
| 4 | 161.4 | 146.7 | 43 / 16 | 14 / 48 | 3.40 | 1.79 |
| 5 | 159.6 | 155.3 | 41 / 16 | 15 / 47 | 3.88 | 1.86 |
| mean | 162.6 | 140.3 | | | | |

The pane build drew 56 or 57 frames a run, with medians of 2.88 to 3.21 ms for a frame with the pane and 1.49 to 1.81 ms without. The island build drew 55 to 64. Peak memory was 497 to 501 MB for both.

## What it says

- Most frames now skip the pane: 39 to 48 of 55 to 64, where the pane build skipped 14 or 15 of 56 or 57. The pane is still rendered about 16 times in 3.2 s. The tray's one-second tick accounts for three of them; the Suggestions the Review streams in and the frame that follows each change of the marks' spots are the other known causes, not counted one by one.
- The whole window's draw time fell by 14 % on average. Unlike the island measurement, the two builds' ranges do not overlap here: 156 to 172 ms for the pane build, 131 to 155 ms for the island build, lower in every pair. It is a small gain, but outside the spread between one build's own runs.
- A frame that skips the pane still costs about 1.7 ms, as before: the workspace is above both islands and is rendered on every frame.
- The island build draws a few more frames in the same 3.2 s, which a cheaper frame allows.

## The workspace during a Refine

Each frame an island asks for renders every view above it, so the workspace is rendered on every frame of any island's motion. A third pair of builds, each with a span at the top of `Workspace::render`, ran a slow Refine three times each, alternating ([`refine.txt`](refine.txt) on the same document, [`refine_stats.py`](refine_stats.py), 4 s from the key that starts the Refine):

| Run | Frames, pane / island | `Workspace::render` calls, pane / island | Time in it (ms), pane / island | Median call (µs), pane / island | Frames with the pane, pane / island | Draw time (ms), pane / island |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 64 / 84 | 64 / 84 | 19.0 / 24.8 | 319 / 338 | 59 / 47 | 243.3 / 257.1 |
| 2 | 65 / 84 | 65 / 84 | 19.9 / 24.9 | 341 / 316 | 60 / 48 | 244.5 / 280.9 |
| 3 | 66 / 71 | 66 / 71 | 19.7 / 21.9 | 270 / 321 | 61 / 66 | 233.1 / 274.7 |

- The workspace is rendered on every frame, as expected: about 0.3 ms to build its elements, some 8 % of the draw time. Its layout, prepaint and paint come on top of that; the span does not time them.
- The tray's island does little for a Refine: most frames still render the pane. The Suggestion's Card shows a "Refining" button whose three dots loop at 30 fps inside the pane. Moving those dots off the pane is the next step of the same kind; until then a Refine renders the pane about as often as before.
- So next to the tray, the workspace matters about as much as the tray did: a Review saves some 22 ms of drawing in 3.2 s from the tray's marks, and the workspace spends about 20 ms in 4 s building its elements. Caching more of the workspace's children, or rendering less in it per frame, is the other half of making these frames cheap.

## Noise

This batch ran at about 3 ms for a frame with the pane, between the two regimes of the first measurement (1.2 ms and 3.5 ms). The pane build's draw time varied by 16 ms between its own runs and the island build's by 25 ms; the difference between the builds' means is 22 ms, and every island run was below every pane run. Read the split of frames by pane render first: it does not depend on the regime.
