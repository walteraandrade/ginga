"""Frames drawn while a fake Review runs, split by whether the editor pane was rendered:
python3 island_stats.py RUN_DIR [RUN_DIR ...]"""
import json, statistics, sys

def stats(folder):
    events = [json.loads(line) for line in open(folder + "/perf.jsonl")]
    keys = [e for e in events if e["k"] == "input" and e.get("name") == "key_down"]
    start = keys[-1]["t"]                      # the Enter that starts the Review
    window = [e for e in events if "t" in e and start <= e["t"] <= start + 3_200_000]
    draws = [e for e in window if e["k"] == "draw"]
    pane = sorted(e["t"] for e in window if e["k"] == "span" and e["name"] == "pane.render")
    with_pane, without = [], []
    for d in draws:
        rendered = any(d["t"] <= t <= d["t"] + d["us"] for t in pane)
        (with_pane if rendered else without).append(d["us"])
    return draws, with_pane, without

def median(xs):
    return statistics.median(xs) / 1000 if xs else float("nan")

print(f"{'run':<10}{'draws':>6}{'draw ms':>9}{'with pane':>11}{'median ms':>11}{'no pane':>9}{'median ms':>11}")
for folder in sys.argv[1:]:
    draws, with_pane, without = stats(folder)
    total = sum(d["us"] for d in draws) / 1000
    print(f"{folder.split('/')[-1]:<10}{len(draws):>6}{total:>9.1f}{len(with_pane):>11}{median(with_pane):>11.2f}{len(without):>9}{median(without):>11.2f}")
