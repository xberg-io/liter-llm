#!/usr/bin/env bash
#MISE description="Run Rust test_app smoke test"
#MISE dir="{{config_root}}/test_apps/rust"
set -euo pipefail

# The only binary is the generated mock-server, so `cargo run` never exercised the
# published crate; run the smoke integration test and refuse an empty suite. ~keep
grep -q '^#\[test\]' tests/smoke_test.rs || {
  echo "test_apps/rust/tests/smoke_test.rs declares no tests; the smoke task would execute nothing" >&2
  exit 1
}
cargo test --test smoke_test
