#!/usr/bin/env bash
set -euo pipefail

BIN="./target/release/compression"
TMPDIR_LOCAL=$(mktemp -d)
trap 'rm -rf "$TMPDIR_LOCAL"' EXIT

PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS+1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL+1)); }

roundtrip() {
    local label="$1" input="$2"
    local compressed="$TMPDIR_LOCAL/compressed"
    local restored="$TMPDIR_LOCAL/restored"
    if $BIN compress   "$input"      "$compressed" >/dev/null 2>&1 \
    && $BIN decompress "$compressed" "$restored"   >/dev/null 2>&1 \
    && cmp -s "$input" "$restored"; then
        pass "$label"
    else
        fail "$label"
    fi
}

expect_error() {
    local label="$1"; shift
    if "$@" >/dev/null 2>&1; then
        fail "$label (expected non-zero exit)"
    else
        pass "$label"
    fi
}

# ── Build ────────────────────────────────────────────────────────────────────
echo "Building..."
cargo build --release -q

# ── Round-trip tests ─────────────────────────────────────────────────────────
echo ""
echo "Round-trip tests"

# Simple ASCII string from the challenge description
echo -n "aaabbc" > "$TMPDIR_LOCAL/simple.txt"
roundtrip "simple string (aaabbc)" "$TMPDIR_LOCAL/simple.txt"

# Single unique byte (edge case: tree is a single leaf)
python3 -c "import sys; sys.stdout.buffer.write(b'A' * 1000)" > "$TMPDIR_LOCAL/single.txt"
roundtrip "single unique byte (1000×A)" "$TMPDIR_LOCAL/single.txt"

# Two unique bytes
python3 -c "import sys; sys.stdout.buffer.write(b'AB' * 500)" > "$TMPDIR_LOCAL/two.txt"
roundtrip "two unique bytes (1000 chars)" "$TMPDIR_LOCAL/two.txt"

# All 256 byte values present
python3 -c "import sys; sys.stdout.buffer.write(bytes(range(256)) * 100)" > "$TMPDIR_LOCAL/all256.bin"
roundtrip "all 256 byte values" "$TMPDIR_LOCAL/all256.bin"

# Binary data (simulated with random-ish bytes)
python3 -c "
import sys, struct
data = bytes((i * 6364136223846793005 + 1442695040888963407) & 0xFF for i in range(65536))
sys.stdout.buffer.write(data)
" > "$TMPDIR_LOCAL/pseudo_random.bin"
roundtrip "pseudo-random binary data (64 KiB)" "$TMPDIR_LOCAL/pseudo_random.bin"

# Single byte file (minimal input)
echo -n "Z" > "$TMPDIR_LOCAL/one_byte.txt"
roundtrip "single byte file" "$TMPDIR_LOCAL/one_byte.txt"

# Skewed distribution (one dominant character — high compression ratio expected)
python3 -c "
import sys
data = b'a' * 9000 + b'b' * 900 + b'c' * 90 + b'd' * 10
sys.stdout.buffer.write(data)
" > "$TMPDIR_LOCAL/skewed.txt"
roundtrip "skewed distribution (9000a 900b 90c 10d)" "$TMPDIR_LOCAL/skewed.txt"

# Les Misérables round-trip (if available)
if [[ -f "les-mis.txt" ]]; then
    roundtrip "Les Misérables (full book)" "les-mis.txt"
fi

# ── Frequency count verification ─────────────────────────────────────────────
echo ""
echo "Frequency count checks"

if [[ -f "les-mis.txt" ]]; then
    count_X=$(python3 -c "print(open('les-mis.txt','rb').read().count(b'X'))")
    count_t=$(python3 -c "print(open('les-mis.txt','rb').read().count(b't'))")
    if [[ "$count_X" -gt 300 && "$count_X" -lt 400 ]]; then
        pass "'X' frequency plausible (got $count_X, spec says 333)"
    else
        fail "'X' frequency out of expected range (got $count_X)"
    fi
    if [[ "$count_t" -gt 200000 && "$count_t" -lt 250000 ]]; then
        pass "'t' frequency plausible (got $count_t, spec says 223000)"
    else
        fail "'t' frequency out of expected range (got $count_t)"
    fi
fi

# ── Compression ratio sanity checks ──────────────────────────────────────────
echo ""
echo "Compression ratio checks"

# Highly skewed text should compress well (below 50%)
skewed_orig=$(wc -c < "$TMPDIR_LOCAL/skewed.txt")
$BIN compress "$TMPDIR_LOCAL/skewed.txt" "$TMPDIR_LOCAL/skewed.huff" >/dev/null 2>&1
skewed_comp=$(wc -c < "$TMPDIR_LOCAL/skewed.huff")
skewed_pct=$(python3 -c "print(f'{$skewed_comp/$skewed_orig*100:.1f}%')")
if [[ $skewed_comp -lt $((skewed_orig / 2)) ]]; then
    pass "skewed data compresses below 50% (got $skewed_pct)"
else
    fail "skewed data did not compress well enough (got $skewed_pct)"
fi

# Les Misérables should compress below 65%
if [[ -f "les-mis.txt" ]]; then
    lm_orig=$(wc -c < "les-mis.txt")
    lm_comp=$(wc -c < "les-mis.huff")
    lm_pct=$(python3 -c "print(f'{$lm_comp/$lm_orig*100:.1f}%')")
    if [[ $lm_comp -lt $((lm_orig * 65 / 100)) ]]; then
        pass "Les Mis compresses below 65% (got $lm_pct)"
    else
        fail "Les Mis compression ratio worse than expected (got $lm_pct)"
    fi
fi

# ── Error handling ────────────────────────────────────────────────────────────
echo ""
echo "Error handling"

expect_error "missing input file" $BIN compress "$TMPDIR_LOCAL/nonexistent.txt" "$TMPDIR_LOCAL/out.huff"
expect_error "invalid compressed file" $BIN decompress "$TMPDIR_LOCAL/simple.txt" "$TMPDIR_LOCAL/out.txt"
expect_error "too few arguments" $BIN compress only_one_arg

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
