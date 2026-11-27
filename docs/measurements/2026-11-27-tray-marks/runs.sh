#!/usr/bin/env bash
# Five pairs of each scenario, in the order P I I P P I I P P I, under Xvfb.
# Usage: RUN_SH=.../scripts/perf/run.sh XDRIVE=.../xdrive BINS=dir bash runs.sh
# BINS holds pane/desafinado and island/desafinado, both release builds with --features perf-probe.
set -u
M=${M:-$(cd "$(dirname "$0")" && pwd)}
R=${RUN_SH:?path to desafinado-rust-port/scripts/perf/run.sh}
export PERF_DIR=${PERF_DIR:-$HOME/.cache/desafinado-perf} XDRIVE=${XDRIVE:?path to xdrive} PERF_DISPLAY=:97
Xvfb :97 -screen 0 1600x1000x24 -nolisten tcp >/dev/null 2>&1 &
X=$!
sleep 1
order="pane island island pane pane island island pane pane island"
for scen in review refine; do
  file=$M/../2026-11-26-ai-caret/scenario.txt; [ $scen = refine ] && file=$M/refine.txt
  declare -A n=([pane]=0 [island]=0)
  for b in $order; do
    n[$b]=$((n[$b]+1))
    bash $R $scen-$b-${n[$b]} $M/../2026-11-26-ai-caret/doc.md $file ${BINS:?folder with pane/ and island/ builds}/$b/desafinado
  done
done
kill $X
