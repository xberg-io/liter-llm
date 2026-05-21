#!/usr/bin/env bash
# Spawn the e2e mock HTTP server, export MOCK_SERVER_URL (plus any
# per-fixture MOCK_SERVER_<ID> URLs the server advertises), run the test
# command passed as argv, and tear the server down on exit.
#
# Used by `alef test --lang <X> --e2e` for languages whose test runners
# don't have a built-in mock-server bootstrap (Dart, Swift, Zig — Python
# has conftest.py, Go has main_test.go's TestMain, C has the Makefile,
# etc.).
#
# Usage:
#   scripts/e2e/run-with-mock-server.sh "cd e2e/dart && dart test"
#
# The first (and only) argument is a shell snippet evaluated with `bash -c`
# inside a process group that inherits the exported env vars.
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <test-command>" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MOCK_SERVER_BIN="${MOCK_SERVER_BIN:-$REPO_ROOT/e2e/rust/target/release/mock-server}"
FIXTURES_DIR="${FIXTURES_DIR:-$REPO_ROOT/fixtures}"

if [ ! -x "$MOCK_SERVER_BIN" ]; then
  echo "mock-server binary not found at $MOCK_SERVER_BIN" >&2
  echo "build it first: cargo build --manifest-path e2e/rust/Cargo.toml --bin mock-server --release" >&2
  exit 1
fi

MOCK_URL_FILE="$(mktemp)"
cleanup() {
  if [ -n "${MOCK_PID:-}" ]; then
    kill "$MOCK_PID" 2>/dev/null || true
  fi
  rm -f "$MOCK_URL_FILE"
}
trap cleanup EXIT

# MOCK_SERVER_NO_STDIN_WATCH=1 swaps the server's stdin-EOF lifetime watch
# for SIGTERM, so the background process survives the bash that spawned it
# closing its stdin (which happens as soon as this script proceeds past
# the `&`).
MOCK_SERVER_NO_STDIN_WATCH=1 "$MOCK_SERVER_BIN" "$FIXTURES_DIR" \
  >"$MOCK_URL_FILE" 2>/dev/null </dev/null &
MOCK_PID=$!

# Wait up to ~10 s for the server to print its URL.
for _ in $(seq 1 200); do
  if grep -q '^MOCK_SERVER_URL=' "$MOCK_URL_FILE" 2>/dev/null; then
    break
  fi
  sleep 0.05
done

if ! grep -q '^MOCK_SERVER_URL=' "$MOCK_URL_FILE" 2>/dev/null; then
  echo "mock-server did not announce a URL within 10s" >&2
  cat "$MOCK_URL_FILE" >&2 || true
  exit 1
fi

MOCK_SERVER_URL="$(grep '^MOCK_SERVER_URL=' "$MOCK_URL_FILE" | head -1 | cut -d= -f2-)"
export MOCK_SERVER_URL

# Some fixtures need a per-fixture base URL (e.g. host-root-route fixtures).
# mock-server emits a `MOCK_SERVERS={"<fixture_id>":"http://..."}` JSON line
# alongside MOCK_SERVER_URL; export each one as MOCK_SERVER_<FIXTURE_ID>.
MOCK_SERVERS_JSON="$(grep -o 'MOCK_SERVERS={.*}' "$MOCK_URL_FILE" | head -1 | cut -d= -f2- || true)"
if [ -n "$MOCK_SERVERS_JSON" ] && command -v python3 >/dev/null 2>&1; then
  while IFS= read -r line; do
    [ -n "$line" ] && export "$line"
  done < <(python3 -c "import json,sys; d=json.loads(sys.argv[1]); print('\n'.join('MOCK_SERVER_{}={}'.format(k.upper().replace('-','_'),v) for k,v in d.items()))" "$MOCK_SERVERS_JSON")
fi

bash -c "$1"
