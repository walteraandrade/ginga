# The AI caret of Desafinado, on gpui-base and on ginga

2026-11-26. The first thing ginga animated in a real app: the AI caret of [desafinado-rust-port](https://github.com/walteraandrade/desafinado-rust-port), which takes off from the writer's caret and glides to each new place in the text in 560 ms.

The same behavior was built twice on top of `origin/main` of the editor, and measured with the editor's own harness (`scripts/perf`, a release build with `--features perf-probe`, Xvfb, the scripted fake model).

| Build | Engine | Change to the editor |
| --- | --- | --- |
| base | `gpui_kit::base::motion::transition`, keyed by element id | [`editor-on-gpui-base.patch`](editor-on-gpui-base.patch) and the new file [`caret_travel-on-gpui-base.rs`](caret_travel-on-gpui-base.rs) |
| ginga | a `ginga::Motion` owned by the editor's plain logic crate, with `ginga_gpui::Clock` | [`editor-on-ginga.patch`](editor-on-ginga.patch) |

## What was measured

The scenario ([`scenario.txt`](scenario.txt) on [`doc.md`](doc.md)) starts a Review. For 3.2 s from the key that starts it, [`glide_stats.py`](glide_stats.py) counts the frames drawn and adds up the time the main thread spent drawing them. Five runs of each build, alternating, both binaries under the same file name.

| Run | Frames, base | Frames, ginga | Draw time, base (ms) | Draw time, ginga (ms) | Median frame, base (ms) | Median frame, ginga (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 57 | 58 | 84.7 | 97.2 | 1.14 | 1.19 |
| 2 | 56 | 57 | 107.0 | 104.7 | 1.30 | 1.67 |
| 3 | 57 | 57 | 68.3 | 67.0 | 1.13 | 1.15 |
| 4 | 57 | 57 | 72.7 | 69.2 | 1.22 | 1.15 |
| 5 | 57 | 57 | 84.9 | 86.0 | 1.16 | 1.16 |
| mean | | | 83.5 | 84.8 | | |

Peak memory was 497 to 499 MB for both.

## What it says

- The two engines cost the same. The means differ by 1.3 ms in about 84 ms, and one build's own runs differ by up to 39 ms.
- A frame of this motion costs the main thread about 1.2 ms. ginga's share of it is 12 to 25 ns (`cargo run --release --example bench`): about 0.002 %.
- So the engine is not where the time goes. GPUI renders the view that asked for the frame, and its ancestors, on every frame of a motion; this app caches no view, so that is the whole window. Making motion cheaper here means drawing less of the window per frame, not computing the value faster.

## What ginga changed in the editor's code

- The glide's logic and its engine now live together in a crate that does not know GPUI, and take the time as an argument. The whole trip is tested as text, with no window (`the_real_trip_covers_most_of_the_way_at_once_and_lands_in_560_ms`).
- The caret starts moving on the frame it shows. On gpui-base the first frame only planted the keyed state, and the trip began on the second.
- The view keeps a clock and asks for a frame while the logic says `moving`. It no longer needs a generation number in an element id to start a new trip.

## Noise that was not explained

The table is the third batch. The first two ran the builds as `desafinado-base` and `desafinado-ginga`, and looked different from it and from each other:

- In those batches ginga peaked 15 MB above base, every time. Run under one file name, the two binaries used the same memory. Twenty minutes later the base binary used the higher figure under either name. The cause was not found.
- The median frame of one binary went from 1.1 ms in one batch to 3.5 ms in another.

Compare builds inside one batch, alternating, under the same file name, and read a difference smaller than the spread between a build's own runs as noise.

## Not measured

- A machine with a GPU. Xvfb draws in software.
- Any motion other than this one.
- Whether a small, separately cached view for the caret would cut the per-frame cost. That is the next experiment.
