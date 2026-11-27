# Recording a gallery page

`examples/gallery/record <page> [folder]` runs `gallery --record` under `xvfb-run`. The app opens the page, waits 1.5 s for the window, starts `ffmpeg -f x11grab` for 2 s, presses Play, presses it again 1 s later so the way back is in the frames too, and then makes a contact sheet of every third frame with ImageMagick's `montage`.

## The trap: waiting blocks drawing

The recording runs in a task on GPUI's main thread (`cx.spawn`). A blocking call there, such as `Child::wait()` on `ffmpeg`, stops GPUI from drawing: every frame then shows the page as it was before Play, and the recording looks like a motion that never started. Wait for the child on the background executor (`background_executor().spawn(async move { grab.wait() })`) and await that.

A recording that shows no motion is this bug before it is a bug in the page: check that the counter on the page ("Shown N times") moved.

## Why a second press

Every page turns around on a second press: the value goes back, the toast leaves, the list adds a row and drops one, the island's dot returns. One press records only the way in; the way out is usually what a change is about.

Related: [[glossary]] (gallery, page), [[codebase/what-a-cached-view-saves]].
