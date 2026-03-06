#!/usr/bin/env bash
set -euo pipefail

BINARY="./target/debug/json-parser"
PASS=0
FAIL=0

check_valid() {
    local file="$1"
    if "$BINARY" "$file" > /dev/null 2>&1; then
        echo "PASS (valid)   $file"
        PASS=$((PASS + 1))
    else
        echo "FAIL (valid)   $file  ← expected valid, got exit 1"
        FAIL=$((FAIL + 1))
    fi
}

check_invalid() {
    local file="$1"
    if ! "$BINARY" "$file" > /dev/null 2>&1; then
        echo "PASS (invalid) $file"
        PASS=$((PASS + 1))
    else
        echo "FAIL (invalid) $file  ← expected invalid, got exit 0"
        FAIL=$((FAIL + 1))
    fi
}

# ── Step 1 ────────────────────────────────────────────────────────────────────
echo "=== Step 1: empty object ==="
check_valid   tests/step1/valid.json
check_invalid tests/step1/invalid.json

# ── Step 2 ────────────────────────────────────────────────────────────────────
echo "=== Step 2: string keys/values ==="
check_valid   tests/step2/valid.json
check_valid   tests/step2/valid2.json
check_invalid tests/step2/invalid.json
check_invalid tests/step2/invalid2.json

# ── Step 3 ────────────────────────────────────────────────────────────────────
echo "=== Step 3: scalars ==="
check_valid   tests/step3/valid.json
check_invalid tests/step3/invalid.json
check_invalid tests/step3/invalid2.json

# ── Step 4 ────────────────────────────────────────────────────────────────────
echo "=== Step 4: nested objects and arrays ==="
check_valid   tests/step4/valid.json
check_valid   tests/step4/valid2.json
check_invalid tests/step4/invalid.json
check_invalid tests/step4/invalid2.json

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
