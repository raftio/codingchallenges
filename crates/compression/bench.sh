#!/usr/bin/env bash
set -euo pipefail

BIN="./target/release/compression"
TMPDIR_LOCAL=$(mktemp -d)
trap 'rm -rf "$TMPDIR_LOCAL"' EXIT
COMPRESSED="$TMPDIR_LOCAL/bench.huff"
RESTORED="$TMPDIR_LOCAL/bench.out"
RUNS=5

# ── Helpers ───────────────────────────────────────────────────────────────────

# Returns elapsed wall-clock milliseconds for a command.
time_ms() {
    local start end
    start=$(python3 -c "import time; print(int(time.time()*1000))")
    "$@" >/dev/null 2>&1
    end=$(python3 -c "import time; print(int(time.time()*1000))")
    echo $((end - start))
}

# Run $RUNS trials, print min/avg/max and throughput.
bench() {
    local label="$1" size_bytes="$2"; shift 2
    local total=0 min=999999 max=0
    for _ in $(seq 1 $RUNS); do
        local ms
        ms=$(time_ms "$@")
        total=$((total + ms))
        [[ $ms -lt $min ]] && min=$ms
        [[ $ms -gt $max ]] && max=$ms
    done
    local avg=$((total / RUNS))
    local mb_s
    mb_s=$(python3 -c "print(f'{$size_bytes/1048576/$avg*1000:.2f}' if $avg > 0 else 'inf')")
    printf "  %-38s  min=%dms  avg=%dms  max=%dms  %s MB/s\n" \
        "$label" "$min" "$avg" "$max" "$mb_s"
}

# ── Build ─────────────────────────────────────────────────────────────────────
echo "Building release binary..."
cargo build --release -q
echo ""

# ── Benchmark each file ───────────────────────────────────────────────────────

run_file_bench() {
    local label="$1" file="$2"
    local size
    size=$(wc -c < "$file")
    echo "$label ($(python3 -c "print(f'{$size/1048576:.2f}') ") MiB, $RUNS runs each):"

    # Pre-compress once so decompression bench has a valid input
    $BIN compress "$file" "$COMPRESSED" >/dev/null 2>&1
    local comp_size
    comp_size=$(wc -c < "$COMPRESSED")
    local ratio
    ratio=$(python3 -c "print(f'{$comp_size/$size*100:.1f}')")
    echo "  Compressed size: $(python3 -c "print(f'{$comp_size/1048576:.2f}') ") MiB (${ratio}% of original)"

    bench "compress" "$size"      $BIN compress   "$file"       "$COMPRESSED"
    bench "decompress" "$comp_size" $BIN decompress "$COMPRESSED" "$RESTORED"
    echo ""
}

# Les Misérables
if [[ -f "les-mis.txt" ]]; then
    run_file_bench "Les Misérables" "les-mis.txt"
fi

# Synthetic: highly compressible (skewed distribution)
python3 -c "
import sys
data = b'a' * 900000 + b'b' * 90000 + b'c' * 9000 + b'd' * 1000
sys.stdout.buffer.write(data)
" > "$TMPDIR_LOCAL/skewed.bin"
run_file_bench "Synthetic skewed (1 MiB)" "$TMPDIR_LOCAL/skewed.bin"

# Synthetic: all 256 byte values uniformly (least compressible)
python3 -c "
import sys
sys.stdout.buffer.write(bytes(range(256)) * 4096)
" > "$TMPDIR_LOCAL/uniform.bin"
run_file_bench "Synthetic uniform 256 bytes (1 MiB)" "$TMPDIR_LOCAL/uniform.bin"
