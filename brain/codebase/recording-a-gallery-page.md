# Recording a gallery page

`crates/ginga-gpui/examples/gallery/record <page> [folder]`, run from anywhere, runs `gallery --record` under `xvfb-run`. The app turns reduced motion off, opens the page and waits for the window's first frame, starts `ffmpeg -f x11grab` for 2 s, waits for ffmpeg's first frame so the page at rest is in it, presses Play, presses it again 1 s later so the way back is in the frames too, and then makes a contact sheet of every third frame with ImageMagick's `montage`. Each wait gives up after 30 s rather than record a blank screen.

## The trap: waiting blocks drawing

The recording runs in a task on GPUI's main thread (`cx.spawn`). A blocking call there, such as `Child::wait()` on `ffmpeg`, stops GPUI from drawing: every frame then shows the page as it was before Play, and the recording looks like a motion that never started. Wait for the child on the background executor (`background_executor().spawn(async move { grab.wait() })`) and await that.

A recording in which no frame changed fails with that question. When it does, suspect this before the page.

## Why a second press

Every page turns around on a second press: the value goes back, the toast leaves, the list adds a row and drops one, the island's dot returns. One press records only the way in; the way out is usually what a change is about.

Related: [[glossary]] (gallery, page), [[codebase/what-a-cached-view-saves]].
