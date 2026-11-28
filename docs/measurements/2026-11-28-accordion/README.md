# The accordion

2026-11-28, for `.opened(id, progress)` (#81). What an opening element makes GPUI do again, and what a frame of many of them costs. Both are measured on GPUI's test platform, with no display, no GPU and no text shaping, so they count what the adapter and GPUI's element tree do on the main thread, not what the driver does.

## What renders again while it opens

`crates/ginga-gpui/tests/opened_cost.rs`, `while_it_opens_the_cached_views_in_it_and_below_it_render_again`. A column 200 px wide holds an answer that opens by a presence's progress (a 160 ms linear tween), with a cached view inside the answer and a cached view below it in the column. A third cached view sits beside the column. Each counts its renders:

| Cached view | Frames of the opening | Renders |
| --- | ---: | ---: |
| inside the opened element | 11 | 11 |
| below it, in the same column | 11 | 10 |
| beside the column | 11 | 0 |

The view inside keeps its place and its size, yet renders on every frame: the cut around it changes, and GPUI reuses a cached view only under the same content mask. The view below is moved by the layout on every frame but the first. Once the answer is fully open and still, nothing renders again and no frame is asked for. The test asserts each of these.

## A frame of many

`time_a_frame_of_many_opening` in the same file, run with `cargo test --release -p ginga-gpui --test opened_cost -- --ignored --nocapture`. A column of questions, each a 20 px line with an answer under it (`p_2` around a 36 px box). Each frame, the view renders again. Three ways to draw the answers:

- **as they are**: shown, with no `.opened`;
- **opened at 1.0**: shown, wrapped in `.opened(..)` at progress 1;
- **opening**: all opening at once, on a tween too long to end during the run.

60 frames each, mean time of a frame. Three runs, on a shared cloud VM (4 vCPUs of an Intel Xeon at 2.1 GHz), Rust 1.97, release build:

| Answers | As they are (µs) | Opened at 1.0 (µs) | Opening (µs) |
| ---: | --- | --- | --- |
| 10 | 58, 58, 65 | 74, 75, 78 | 74, 76, 76 |
| 100 | 572, 595, 875 | 817, 828, 1,359 | 804, 806, 840 |
| 400 | 3,042, 2,959, 3,063 | 4,225, 4,434, 4,233 | 4,244, 4,165, 4,094 |

The third run's 100 row was disturbed: its "as they are" and "opened at 1.0" are both about half again as slow as in the other runs.

## What it says

- An opened element costs about 3 µs a frame more than the element as it is, here 1.4 times as much for this small answer: its box is one more layout node, and it reads and writes its element state twice. Fully open and opening cost the same; the cut does not add.
- So drawing 400 answers in a frame that renders them all costs about 1.2 ms more. An accordion usually has a few answers, and only while one moves are frames drawn.
- The larger cost of an opening is outside it: cached views in it, and those it moves, render again on each frame. Keep heavy views beside the column that opens, not below it, or in an [island](../2026-11-27-island-alone/README.md).
