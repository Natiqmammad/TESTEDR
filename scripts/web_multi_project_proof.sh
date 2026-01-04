#!/usr/bin/env bash
# web_multi_project_proof.sh - Verify project isolation
#
# This script verifies that:
# 1. Building project A produces UI with "Hello A"
# 2. Building project B produces UI with "Hello B"
# 3. The UIs are different (not stale cache)

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
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

echo ""
echo "========================================"
echo "  Multi-Project Isolation Test"
echo "========================================"
echo ""

# Create Project A
info "Creating Project A with 'Hello Project A'..."
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
    ui.window("Project A", fun(ctx) {
        ctx.text("Hello Project A");
        ctx.button("Button A", fun() { });
    });
}
EOF

# Create Project B
info "Creating Project B with 'Hello Project B'..."
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
    ui.window("Project B", fun(ctx) {
        ctx.text("Hello Project B");
        ctx.button("Button B", fun() { });
    });
}
EOF

# Build Project A
info "Building Project A..."
(cd "$tmpdir/project_a" && "$APEXRC" build --target web > "$tmpdir/build_a.log" 2>&1)

PROJECT_ID_A=$(grep -oP 'projectId=\K[a-f0-9]+' "$tmpdir/build_a.log" | head -1)
BUILD_ID_A=$(grep -oP 'buildId=\K[a-f0-9]+' "$tmpdir/build_a.log" | head -1)
MAIN_HASH_A=$(grep -oP 'mainHash=\K[a-f0-9]+' "$tmpdir/build_a.log" | head -1)

echo "  Project A:"
echo "    projectId: $PROJECT_ID_A"
echo "    buildId:   $BUILD_ID_A"
echo "    mainHash:  $MAIN_HASH_A"

# Check Project A bytecode contains source
AFBC_PATH_A="$tmpdir/project_a/target/web/$PROJECT_ID_A/$BUILD_ID_A/afns_app.$BUILD_ID_A.afbc"
if [[ -f "$AFBC_PATH_A" ]] && grep -q "Hello Project A" "$AFBC_PATH_A" 2>/dev/null; then
    pass "Project A bytecode contains source"
else
    fail "Project A bytecode does NOT contain source! Path: $AFBC_PATH_A"
fi

# Check Project A manifest
MANIFEST_PATH_A="$tmpdir/project_a/target/web/$PROJECT_ID_A/$BUILD_ID_A/afns_manifest.json"
MANIFEST_PROJECT_ID_A=$(grep -oP '"projectId":\s*"\K[^"]+' "$MANIFEST_PATH_A" 2>/dev/null)
if [[ "$MANIFEST_PROJECT_ID_A" == "$PROJECT_ID_A" ]]; then
    pass "Project A manifest has correct projectId"
else
    fail "Project A manifest projectId mismatch"
fi

echo ""

# Build Project B
info "Building Project B..."
(cd "$tmpdir/project_b" && "$APEXRC" build --target web > "$tmpdir/build_b.log" 2>&1)

PROJECT_ID_B=$(grep -oP 'projectId=\K[a-f0-9]+' "$tmpdir/build_b.log" | head -1)
BUILD_ID_B=$(grep -oP 'buildId=\K[a-f0-9]+' "$tmpdir/build_b.log" | head -1)
MAIN_HASH_B=$(grep -oP 'mainHash=\K[a-f0-9]+' "$tmpdir/build_b.log" | head -1)

echo "  Project B:"
echo "    projectId: $PROJECT_ID_B"
echo "    buildId:   $BUILD_ID_B"
echo "    mainHash:  $MAIN_HASH_B"

# Check Project B bytecode contains source
AFBC_PATH_B="$tmpdir/project_b/target/web/$PROJECT_ID_B/$BUILD_ID_B/afns_app.$BUILD_ID_B.afbc"
if [[ -f "$AFBC_PATH_B" ]] && grep -q "Hello Project B" "$AFBC_PATH_B" 2>/dev/null; then
    pass "Project B bytecode contains source"
else
    fail "Project B bytecode does NOT contain source! Path: $AFBC_PATH_B"
fi

# Verify isolation - projects should have different IDs
echo ""
info "Verifying project isolation..."

if [[ "$PROJECT_ID_A" == "$PROJECT_ID_B" ]]; then
    fail "Projects have SAME projectId - isolation FAILED"
fi
pass "Projects have different projectIds"

if [[ "$BUILD_ID_A" == "$BUILD_ID_B" ]]; then
    fail "Projects have SAME buildId - isolation FAILED"
fi
pass "Projects have different buildIds"

if [[ "$MAIN_HASH_A" == "$MAIN_HASH_B" ]]; then
    fail "Projects have SAME mainHash - isolation FAILED"
fi
pass "Projects have different mainHashes"

# Verify bytecodes are different
if [[ ! -f "$AFBC_PATH_A" ]] || [[ ! -f "$AFBC_PATH_B" ]]; then
    fail "Bytecode files not found"
fi

AFBC_HASH_A=$(sha256sum "$AFBC_PATH_A" | cut -d' ' -f1)
AFBC_HASH_B=$(sha256sum "$AFBC_PATH_B" | cut -d' ' -f1)

if [[ "$AFBC_HASH_A" == "$AFBC_HASH_B" ]]; then
    fail "Projects have SAME bytecode hash - isolation FAILED"
fi
pass "Projects have different bytecode files"

echo ""
echo "========================================"
echo -e "  ${GREEN}✓ Multi-Project Isolation: PASSED${NC}"
echo "========================================"
echo ""
echo "Verified:"
echo "  - Project A: projectId=$PROJECT_ID_A, mainHash=$MAIN_HASH_A"
echo "  - Project B: projectId=$PROJECT_ID_B, mainHash=$MAIN_HASH_B"
echo "  - All IDs and hashes are different"
echo "  - Bytecode files contain full source code"
