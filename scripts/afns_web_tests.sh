#!/usr/bin/env bash
# afns_web_tests.sh - Automated tests for AFNS web build
#
# Tests all examples and verifies build output

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APEXRC="$ROOT/target/debug/apexrc"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; }
info() { echo -e "${YELLOW}→${NC} $1"; }

# Ensure apexrc is built
if [[ ! -x "$APEXRC" ]]; then
    info "Building apexrc..."
    (cd "$ROOT" && cargo build --bin apexrc --quiet)
fi

TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_example() {
    local name="$1"
    
    info "Testing $name..."
    
    local example_dir="$ROOT/examples/$name"
    
    if [[ ! -d "$example_dir" ]]; then
        echo -e "${RED}  Example directory not found: $example_dir${NC}"
        ((TESTS_FAILED++)) || true
        return
    fi
    
    # Build (in subshell to avoid cd issues)
    if ! (cd "$example_dir" && "$APEXRC" build --target web > /tmp/build_${name}.log 2>&1); then
        echo -e "${RED}  Build failed${NC}"
        cat /tmp/build_${name}.log
        ((TESTS_FAILED++)) || true
        return
    fi
    
    # Check output files
    local target="$example_dir/target/web"
    
    if [[ ! -f "$target/index.html" ]]; then
        fail "  index.html not found"
        ((TESTS_FAILED++)) || true
        return
    fi
    
    if [[ ! -f "$target/afns_manifest.json" ]]; then
        fail "  manifest not found"
        ((TESTS_FAILED++)) || true
        return
    fi
    
    # Check bytecode exists
    local afbc_count=$(ls -1 "$target"/*.afbc 2>/dev/null | wc -l)
    if [[ "$afbc_count" -lt 1 ]]; then
        fail "  No .afbc file found"
        ((TESTS_FAILED++)) || true
        return
    fi
    
    # Check for expected text in source (verifies source was read)
    if grep -q "buildId" "$target/afns_manifest.json"; then
        pass "$name - Build OK"
        ((TESTS_PASSED++)) || true
    else
        fail "$name - Manifest missing buildId"
        ((TESTS_FAILED++)) || true
    fi
    
    # Extract and display build info
    local build_id=$(grep -oP '"buildId":\s*"\K[^"]+' "$target/afns_manifest.json")
    echo "    buildId: $build_id"
}

# Run tests
echo ""
echo "========================================"
echo "  AFNS Web Build Tests"
echo "========================================"
echo ""

test_example "ui_hello"
test_example "ui_counter"
test_example "ui_layout"
test_example "ui_all_widgets"

echo ""
echo "========================================"
echo "  Results: $TESTS_PASSED passed, $TESTS_FAILED failed"
echo "========================================"

if [[ "$TESTS_FAILED" -gt 0 ]]; then
    exit 1
fi

echo ""
pass "All tests passed!"
