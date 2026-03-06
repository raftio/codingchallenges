#!/usr/bin/env bash
set -euo pipefail

FILE="test.txt"
BIN="./target/release/wc"
RUNS=100

time_cmd() {
  local start end elapsed
  start=$(date +%s%N)
  for _ in $(seq 1 $RUNS); do
    "$@" > /dev/null 2>&1
  done
  end=$(date +%s%N)
  echo $(( (end - start) / 1000000 ))
}

printf "%-12s %-12s %-12s %s\n" "flag" "ccwc(ms)" "wc(ms)" "ratio(ccwc/wc)"
printf "%-12s %-12s %-12s %s\n" "----" "--------" "------" "--------------"

for flag in -c -l -w -m ""; do
  label=${flag:-default}

  if [[ -z "$flag" ]]; then
    t_ccwc=$(time_cmd "$BIN" "$FILE")
    t_wc=$(time_cmd wc "$FILE")
  else
    t_ccwc=$(time_cmd "$BIN" $flag "$FILE")
    t_wc=$(time_cmd wc $flag "$FILE")
  fi

  ratio=$(awk "BEGIN { printf \"%.2f\", $t_ccwc / ($t_wc == 0 ? 1 : $t_wc) }")
  printf "%-12s %-12s %-12s %s\n" "$label" "$t_ccwc" "$t_wc" "$ratio"
done
