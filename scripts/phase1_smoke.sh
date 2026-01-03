#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLES=(
    "examples/minimal_hello"
    "examples/control_flow_if"
    "examples/math_basic"
    "examples/loop_break_continue"
    "examples/switch_match_basic"
)

for dir in "${EXAMPLES[@]}"; do
    echo "[phase1-smoke] $dir"
    (cd "$ROOT/$dir" && apexrc check)
    (cd "$ROOT/$dir" && apexrc build)
    (cd "$ROOT/$dir" && apexrc run)
done

echo "[phase1-smoke] all examples passed"
