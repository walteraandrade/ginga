"""Frames drawn from the last key down, split by whether the editor pane was
rendered in them, and the workspace's renders:
python3 -I tray_stats.py WINDOW_MS RUN_DIR [RUN_DIR ...]"""
import json, statistics, sys

def stats(folder, window_us):
    events = [json.loads(line) for line in open(folder + "/perf.jsonl")]
    keys = [e for e in events if e["k"] == "input" and e.get("name") == "key_down"]
    start = keys[-1]["t"]
    window = [e for e in events if "t" in e and start <= e["t"] <= start + window_us]
    draws = [e for e in window if e["k"] == "draw"]
    pane = sorted(e["t"] for e in window if e["k"] == "span" and e["name"] == "pane.render")
    ws = [e["us"] for e in window if e["k"] == "span" and e["name"] == "workspace.render"]
    with_pane, without = [], []
    for d in draws:
        rendered = any(d["t"] <= t <= d["t"] + d["us"] for t in pane)
        (with_pane if rendered else without).append(d["us"])
    return draws, with_pane, without, ws

def median(xs):
    return statistics.median(xs) / 1000 if xs else float("nan")

window_us = int(sys.argv[1]) * 1000
print(f"{'run':<12}{'draws':>6}{'draw ms':>9}{'with pane':>10}{'med ms':>8}{'no pane':>8}{'med ms':>8}{'ws calls':>9}{'ws ms':>7}")
for folder in sys.argv[2:]:
    draws, with_pane, without, ws = stats(folder, window_us)
    total = sum(d["us"] for d in draws) / 1000
    print(f"{folder.split('/')[-1]:<12}{len(draws):>6}{total:>9.1f}{len(with_pane):>10}{median(with_pane):>8.2f}{len(without):>8}{median(without):>8.2f}{len(ws):>9}{sum(ws)/1000:>7.1f}")
