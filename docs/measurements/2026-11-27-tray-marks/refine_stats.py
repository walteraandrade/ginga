"""Frames, workspace and pane renders while a slow Refine runs (4 s from the
Enter that starts it): python3 -I refine_stats.py RUN_DIR [RUN_DIR ...]

The measured builds carry the span `workspace.render` twice, nested, at the top
of `Workspace::render`; renders are counted once each, by start time."""
import json
import statistics
import sys

WINDOW_US = 4_000_000


def spans(window, name):
    found = sorted((e["t"], e["us"]) for e in window if e["k"] == "span" and e["name"] == name)
    once = []
    for t, us in found:
        if once and abs(t - once[-1][0]) < 50 and name == "workspace.render":
            once[-1] = (once[-1][0], max(once[-1][1], us))
            continue
        once.append((t, us))
    return once


def stats(folder):
    events = [json.loads(line) for line in open(folder + "/perf.jsonl")]
    keys = [e for e in events if e["k"] == "input" and e.get("name") == "key_down"]
    start = keys[-1]["t"]
    window = [e for e in events if "t" in e and start <= e["t"] <= start + WINDOW_US]
    draws = [e["us"] for e in window if e["k"] == "draw"]
    workspace = spans(window, "workspace.render")
    pane = spans(window, "pane.render")
    return draws, workspace, pane


print(f"{'run':<22}{'draws':>6}{'draw ms':>9}{'ws renders':>11}{'ws ms':>8}{'ws median us':>13}{'pane renders':>13}{'pane ms':>9}")
for folder in sys.argv[1:]:
    draws, workspace, pane = stats(folder)
    ws_us = [us for _, us in workspace]
    pane_us = [us for _, us in pane]
    print(
        f"{folder.split('/')[-1]:<22}{len(draws):>6}{sum(draws) / 1000:>9.1f}"
        f"{len(workspace):>11}{sum(ws_us) / 1000:>8.1f}{statistics.median(ws_us) if ws_us else 0:>13.0f}"
        f"{len(pane):>13}{sum(pane_us) / 1000:>9.1f}"
    )
