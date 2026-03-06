#!/usr/bin/env bash
set -euo pipefail

FILE="test.txt"
BIN="./target/release/wc"

for flag in -c -l -w -m; do
  echo "=== $flag ==="
  diff <($BIN $flag "$FILE") <(wc $flag "$FILE") && echo "OK" || echo "DIFF!"
done

echo "=== default ==="
diff <($BIN "$FILE") <(wc "$FILE") && echo "OK" || echo "DIFF!"

echo "=== stdin -l ==="
diff <(cat "$FILE" | $BIN -l) <(cat "$FILE" | wc -l) && echo "OK" || echo "DIFF!"
