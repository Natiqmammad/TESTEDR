#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_MANIFEST="$ROOT/nightscript-server/Cargo.toml"
REGISTRY_URL="http://127.0.0.1:5665"

tmpdir="$(mktemp -d)"
cleanup() {
    rm -rf "$tmpdir"
    if [[ -n "${SERVER_PID:-}" ]]; then
        kill "$SERVER_PID" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

echo "[phase1] starting registry server..."
(cd "$ROOT/nightscript-server" && cargo run --quiet --manifest-path "$SERVER_MANIFEST") >"$tmpdir/server.log" 2>&1 &
SERVER_PID=$!
sleep 2

echo "[phase1] registering demo user..."
curl -sSf -X POST "$REGISTRY_URL/api/v1/register" \
    -H "Content-Type: application/json" \
    -d '{"username":"demo","email":"demo@example.com","password":"secret"}' >/dev/null

printf "demo\nsecret\n" | (cd "$ROOT" && apexrc login --registry "$REGISTRY_URL") >/dev/null

LIB_DIR="$tmpdir/hello-afml"
APP_DIR="$tmpdir/example-app"
mkdir -p "$LIB_DIR/src" "$APP_DIR/src"

cat >"$LIB_DIR/Apex.toml" <<'TOML'
[package]
name = "hello-afml"
version = "0.1.0"
language = "afml"
description = "Phase 1 demo lib"
license = "MIT"
[dependencies]
[registry]
url = "http://127.0.0.1:5665"
TOML

cat >"$LIB_DIR/src/lib.afml" <<'AFML'
import forge.log as log;

fun greet() {
    log.info("hi from registry");
}
AFML

echo "[phase1] publishing library..."
(cd "$LIB_DIR" && apexrc publish --registry "$REGISTRY_URL")

cat >"$APP_DIR/Apex.toml" <<'TOML'
[package]
name = "registry-app"
version = "0.1.0"
language = "afml"
[registry]
url = "http://127.0.0.1:5665"
TOML

cat >"$APP_DIR/src/main.afml" <<'AFML'
import hello-afml as hello;

fun apex() {
    hello.greet();
}
AFML

echo "[phase1] installing deps..."
(cd "$APP_DIR" && apexrc add hello-afml@^0.1.0)
(cd "$APP_DIR" && apexrc install)
(cd "$APP_DIR" && apexrc tree)
(cd "$APP_DIR" && apexrc why hello-afml | tee "$tmpdir/why.txt")
grep -q "hello-afml" "$tmpdir/why.txt"
(cd "$APP_DIR" && apexrc install --locked)

echo "[phase1] building and running..."
(cd "$APP_DIR" && apexrc build)
output="$(cd "$APP_DIR" && apexrc run)"
echo "$output"
grep -q "hi from registry" <<<"$output"

echo "[phase1] success"
