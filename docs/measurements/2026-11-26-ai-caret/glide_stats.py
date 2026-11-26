"""Frames drawn while a fake Review runs: python3 glide_stats.py RUN_DIR [RUN_DIR ...]"""
import collections, json, statistics, sys

def stats(folder):
    events = [json.loads(line) for line in open(folder + "/perf.jsonl")]
    keys = [e for e in events if e["k"] == "input" and e.get("name") == "key_down"]
    start = keys[-1]["t"]                      # the Enter that starts the Review
    window = [e for e in events if "t" in e and start <= e["t"] <= start + 3_200_000]
    draws = [e for e in window if e["k"] == "draw"]
    spans = collections.defaultdict(list)
    for e in window:
        if e["k"] == "span":
            spans[e["name"]].append(e["us"])
    views = collections.defaultdict(list)
    for e in window:
        if e["k"] == "view":
            views[(e["name"].split("::")[-1], e["x"]["phase"])].append(e["x"]["self_us"])
    return draws, spans, views

rows = []
for folder in sys.argv[1:]:
    draws, spans, views = stats(folder)
    us = [d["us"] for d in draws]
    pane = spans.get("pane.render", [])
    rows.append((folder.split("/")[-1], len(us), sum(us) / 1000, statistics.median(us) / 1000 if us else 0,
                 len(pane), sum(pane) / 1000))
    top = sorted(views.items(), key=lambda kv: -sum(kv[1]))[:6]
    print(folder.split("/")[-1], "top view self time (ms):",
          ", ".join(f"{name}/{phase} {sum(v)/1000:.1f}" for (name, phase), v in top))
print(f"{'run':<12} {'draws':>5} {'draw ms':>8} {'median ms':>9} {'pane.render':>11} {'pane ms':>8}")
for name, n, total, median, pane_n, pane_ms in rows:
    print(f"{name:<12} {n:>5} {total:>8.1f} {median:>9.2f} {pane_n:>11} {pane_ms:>8.1f}")
