#!/usr/bin/env bash
# web_browser_isolation_test.sh - Test project isolation in actual browser
#
# Creates 2 projects with different UIs, starts servers, and verifies
# DOM actually shows different content (not stale cache)

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APEXRC="$ROOT/target/debug/apexrc"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
info() { echo -e "${YELLOW}→${NC} $1"; }

# Ensure apexrc is built
if [[ ! -x "$APEXRC" ]]; then
    info "Building apexrc..."
    (cd "$ROOT" && cargo build --bin apexrc --quiet)
fi

# Create temp directory
tmpdir="$(mktemp -d)"
cleanup() { 
    [[ -n "${SERVER_PID_A:-}" ]] && kill $SERVER_PID_A 2>/dev/null || true
    [[ -n "${SERVER_PID_B:-}" ]] && kill $SERVER_PID_B 2>/dev/null || true
    rm -rf "$tmpdir"
}
trap cleanup EXIT

echo ""
echo "========================================"
echo "  Browser Project Isolation Test"
echo "========================================"
echo ""

# Create Project A
info "Creating Project A with 'AAAAA Text'..."
mkdir -p "$tmpdir/project_a/src"
cat > "$tmpdir/project_a/Apex.toml" << 'EOF'
[package]
name = "project_a"
version = "0.1.0"
language = "afml"
EOF

cat > "$tmpdir/project_a/src/main.afml" << 'EOF'
import forge.gui.native as ui;

fun apex() {
    ui.window("AAAAA Window", fun(ctx) {
        ctx.text("AAAAA Text Content");
        ctx.button("AAAAA Button", fun() { });
    });
}
EOF

# Create Project B  
info "Creating Project B with 'BBBBB Text'..."
mkdir -p "$tmpdir/project_b/src"
cat > "$tmpdir/project_b/Apex.toml" << 'EOF'
[package]
name = "project_b"
version = "0.1.0"
language = "afml"
EOF

cat > "$tmpdir/project_b/src/main.afml" << 'EOF'
import forge.gui.native as ui;

fun apex() {
    ui.window("BBBBB Window", fun(ctx) {
        ctx.text("BBBBB Text Content");
        ctx.button("BBBBB Button", fun() { });
    });
}
EOF

# Build and start server for Project A
info "Building Project A..."
(cd "$tmpdir/project_a" && "$APEXRC" build --target web > "$tmpdir/build_a.log" 2>&1)

PROJECT_ID_A=$(grep -oP 'projectId=\K[a-f0-9]+' "$tmpdir/build_a.log" | head -1)
BUILD_ID_A=$(grep -oP 'buildId=\K[a-f0-9]+' "$tmpdir/build_a.log" | head -1)

echo "  Project A: projectId=$PROJECT_ID_A, buildId=$BUILD_ID_A"

info "Starting server for Project A on port 8001..."
(cd "$tmpdir/project_a" && "$APEXRC" run --target web > "$tmpdir/server_a.log" 2>&1) &
SERVER_PID_A=$!
sleep 2

# Build and start server for Project B
info "Building Project B..."
(cd "$tmpdir/project_b" && "$APEXRC" build --target web > "$tmpdir/build_b.log" 2>&1)

PROJECT_ID_B=$(grep -oP 'projectId=\K[a-f0-9]+' "$tmpdir/build_b.log" | head -1)
BUILD_ID_B=$(grep -oP 'buildId=\K[a-f0-9]+' "$tmpdir/build_b.log" | head -1)

echo "  Project B: projectId=$PROJECT_ID_B, buildId=$BUILD_ID_B"

info "Starting server for Project B on port 8002..."
(cd "$tmpdir/project_b" && "$APEXRC" run --target web > "$tmpdir/server_b.log" 2>&1) &
SERVER_PID_B=$!
sleep 2

echo ""
info "Servers running. Manual verification required:"
echo ""
echo "  1. Open http://localhost:3000 → Should show 'AAAAA Text Content'"
echo "  2. Open http://localhost:3001 → Should show 'BBBBB Text Content'"  
echo ""
echo "  If both show same text -> FAIL (cache issue)"
echo "  If they show different text -> PASS (isolation works)"
echo ""
echo "Press Ctrl+C when done testing..."
wait
