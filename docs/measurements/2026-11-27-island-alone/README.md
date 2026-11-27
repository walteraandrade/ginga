# The island on its own

2026-11-27. The two measurements before this one ([the AI caret](../2026-11-26-ai-caret/README.md), [the caret in an island](../2026-11-26-ai-caret-island/README.md)) ran inside Desafinado's harness and were noisy: one binary's median frame went from 1.1 ms to 3.5 ms between batches. This one measures the island with nothing else around it.

## What was measured

[`crates/ginga-gpui/examples/island_cost/`](../../../crates/ginga-gpui/examples/island_cost/) opens an 800 × 600 window with a view made heavy on purpose: 2,400 boxes, each with a number in it. A dot goes back and forth across the window with `Spring::SMOOTH`, turning around each time it arrives, so it never stops asking for frames. There are two modes:

| Mode | Where the dot is drawn | The heavy view |
| --- | --- | --- |
| `inside` | by the heavy view itself | rendered again on every frame of the dot |
| `island` | a `ginga_gpui::Island` beside the heavy view | embedded with `.cached(..)` |

Each run waits for its first frame and 1 s more, then measures for 5 s: how many frames the dot was drawn in, how many times the heavy view rendered, the main thread's CPU time from `/proc/thread-self/schedstat`, and, from batch 3 on, the CPU time of all the process's threads, the software driver's included. The main thread's time covers everything it did: render, layout, paint, and handing the scene to the driver. Each run is a separate process that turns reduced motion off for itself, and the parent refuses a run that failed or drew no frame, and an island run in which the heavy view rendered. The modes alternate, five runs each.

```sh
# Ubuntu 24.04: a virtual display, a software Vulkan driver, and what GPUI's Linux platform links.
sudo apt-get install xvfb mesa-vulkan-drivers libxkbcommon-x11-dev libwayland-dev libfontconfig-dev libfreetype-dev
xvfb-run -a -s "-screen 0 1280x1024x24" cargo run --release -p ginga-gpui --example island_cost
```

The machine was a shared cloud VM: 4 vCPUs of an Intel Xeon at 2.1 GHz, Linux 6.18, no GPU. The display was Xvfb 21.1.12, and wgpu drew through Mesa 25.2.8's lavapipe, a software Vulkan driver. Rust 1.97 for batches 1 and 2, 1.95 for batch 3; release builds. The full output is in [`output.txt`](output.txt).

## Results

Three batches on the same machine. Batch 3 ran the example as committed. Batches 1 and 2 ran it before the review: its 2,400 cells each carried an element id, and the measurement started a fixed second after the window opened. Batch 1 also read a run's line another way, which changed nothing about what is measured.

| Batch, run | inside: frames | inside: heavy renders | inside: ms/frame | island: frames | island: heavy renders | island: ms/frame |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1, 1 | 72 | 72 | 46.6 | 103 | 0 | 19.8 |
| 1, 2 | 63 | 63 | 54.1 | 104 | 0 | 23.0 |
| 1, 3 | 66 | 66 | 50.5 | 103 | 0 | 20.8 |
| 1, 4 | 60 | 60 | 56.1 | 102 | 0 | 21.7 |
| 1, 5 | 68 | 68 | 49.8 | 104 | 0 | 20.5 |
| 2, 1 | 74 | 74 | 44.4 | 105 | 0 | 19.5 |
| 2, 2 | 76 | 76 | 42.8 | 94 | 0 | 22.7 |
| 2, 3 | 70 | 70 | 47.6 | 105 | 0 | 19.0 |
| 2, 4 | 66 | 66 | 51.2 | 104 | 0 | 18.8 |
| 2, 5 | 71 | 71 | 48.1 | 105 | 0 | 18.8 |
| 3, 1 | 67 | 67 | 49.6 | 103 | 0 | 21.0 |
| 3, 2 | 74 | 74 | 43.1 | 103 | 0 | 23.7 |
| 3, 3 | 67 | 67 | 49.3 | 103 | 0 | 23.0 |
| 3, 4 | 66 | 66 | 50.9 | 103 | 0 | 22.4 |
| 3, 5 | 71 | 71 | 45.9 | 103 | 0 | 21.9 |

| | inside: ms/frame | island: ms/frame |
| --- | ---: | ---: |
| batch 1: median, range | 50.5 (46.6 to 56.1) | 20.8 (19.8 to 23.0) |
| batch 2: median, range | 47.6 (42.8 to 51.2) | 19.0 (18.8 to 22.7) |
| batch 3: median, range | 49.3 (43.1 to 50.9) | 22.4 (21.0 to 23.7) |
| batch 3, all threads: median | 85.4 | 59.2 |

## What it says

- With the island, the heavy view never rendered while the dot moved: 0 times in every run of the three batches. Inside it, the heavy view rendered on every frame.
- A frame cost the main thread about 19 to 22 ms with the island against 48 to 50 ms without it (the medians of the three batches), some 55 to 60 % less. The ranges don't overlap: the island's worst run (23.7 ms) is about half of the inside mode's best (42.8 ms).
- With the frames cheaper, more of them were drawn in the same 5 s: about 20 a second against about 14.
- About 20 ms of the main thread's frame is left when the heavy view is skipped. It is not the heavy view's own work: a cached view is not rendered, laid out or painted again, GPUI copies what it painted last frame ([what a cached view saves](../../../brain/codebase/what-a-cached-view-saves.md)). What is left is the scene with all its primitives going to the GPU every frame: building and encoding it and handing it to the driver. With all threads counted, a frame cost about 59 ms with the island against 85 without it: the software driver drew the pixels on its own threads, and that part changed less. On this machine the island halves the main thread's frame. It doesn't make it small.

## What these numbers do not cover

- **A machine with a GPU.** Everything was drawn in software. With a real GPU, the share of the frame left after skipping the heavy view should be much smaller, so the relative gain is likely larger, but this run doesn't show it.
- **macOS and Windows.** Only Linux on X11 (Xvfb) was measured.
- **A real app.** The heavy view is synthetic: one flat grid of boxes with text. Desafinado's pane has deeper trees, more text and views above the island that aren't cached. In its measurement the island saved about 40 % on the frames that skipped the pane, but only one frame in four skipped it.
- **Memory**, and **frame times as a user sees them**. CPU time was counted, the main thread's and all threads'. Frame pacing and latency weren't.
- **Noise across machines.** All batches ran on one shared VM. Within a batch, a mode's runs spread by up to 20 %. The second batch sat about 6 % lower than the first in both modes, and the third, whose cells carry no ids, sat between them. A batch on another day or machine may sit higher or lower as a whole, as Desafinado's did. The gap between the modes was far larger than either spread.
