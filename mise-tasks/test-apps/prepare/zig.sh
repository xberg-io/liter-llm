#!/usr/bin/env bash
#MISE description="Resolve the current published Zig archive and its content hash"
#MISE dir="{{config_root}}/test_apps/zig"
set -euo pipefail

version=$(python3 -c "import sys, tomllib; print(tomllib.load(open(sys.argv[1], 'rb'))['workspace']['package']['version'])" \
  "$MISE_CONFIG_ROOT/Cargo.toml")
zig fetch --save=liter_llm \
  "https://github.com/xberg-io/liter-llm/releases/download/v${version}/liter-llm-zig-v${version}.tar.gz"
