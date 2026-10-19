#!/usr/bin/env bash
# Run scripts/php/smoke.php against the locally built liter_llm extension.
#
# The extension is LITER_LLM_PHP_EXT when set, otherwise the release cdylib
# from `task php:build`. PHP selects the interpreter (default: php). A missing
# extension, a version mismatch or a failing check exits non-zero.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
php_bin="${PHP:-php}"

ext="${LITER_LLM_PHP_EXT:-}"
if [[ -z "$ext" ]]; then
  for candidate in \
    "$root/target/release/libliter_llm_php.so" \
    "$root/target/release/libliter_llm_php.dylib" \
    "$root/crates/liter-llm-php/target/release/libliter_llm_php.so" \
    "$root/crates/liter-llm-php/target/release/libliter_llm_php.dylib"; do
    if [[ -f "$candidate" ]]; then
      ext="$candidate"
      break
    fi
  done
fi
if [[ -z "$ext" || ! -f "$ext" ]]; then
  echo "php smoke: liter_llm extension not found${ext:+ at $ext}; run \`task php:build\` or set LITER_LLM_PHP_EXT" >&2
  exit 1
fi

version="$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["workspace"]["package"]["version"])' "$root/Cargo.toml")"

# ~keep -n ignores php.ini: a globally installed liter_llm would otherwise be "already loaded",
# and the check would silently examine that older build instead of the one under test.
exec "$php_bin" -n -d "extension=$ext" "$root/scripts/php/smoke.php" "$version"
