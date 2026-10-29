#!/usr/bin/env bash
#MISE description="Build WASM package (web target)"
#USAGE arg "[target]" default="web" help="wasm-pack target: web, bundler or nodejs (nodejs writes pkg/nodejs)"
set -euo pipefail

# An exported CC_wasm32_unknown_unknown or WASI_SYSROOT (install-wasi-sdk sets both in CI) wins;
# otherwise WASM_CC and WASM_SYSROOT pick them, defaulting to Homebrew's llvm and wasi-libc. ~keep
export CC_wasm32_unknown_unknown="${CC_wasm32_unknown_unknown:-${WASM_CC:-/opt/homebrew/opt/llvm/bin/clang}}"
export WASI_SYSROOT="${WASI_SYSROOT:-${WASM_SYSROOT:-/opt/homebrew/share/wasi-sysroot}}"
target="${usage_target:-web}"
args=(--target "$target" --release)
if [ "$target" = nodejs ]; then
  args+=(--out-dir pkg/nodejs)
fi
wasm-pack build "$MISE_CONFIG_ROOT/crates/liter-llm-wasm" "${args[@]}"
