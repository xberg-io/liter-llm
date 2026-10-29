#!/usr/bin/env bash
#MISE description="Run generated Bats Homebrew consumer checks"
#MISE dir="{{config_root}}/test_apps/homebrew"
set -euo pipefail

command -v bats >/dev/null || {
  echo "bats is required: mise install bats" >&2
  exit 1
}
test -f homebrew.bats || {
  echo "Generate the Homebrew suite with alef e2e generate --registry --lang homebrew" >&2
  exit 1
}
bash run_tests.sh
