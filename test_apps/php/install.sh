#!/usr/bin/env bash
# alef-generated installer for registry-mode PHP test_app.
# Installs the kreuzberg/liter-llm extension via PIE before `composer install` runs.
# Requires `php` on PATH; downloads and runs PIE if needed.
# Version is alef-injected at generate time so the script is self-contained.
set -euo pipefail

# Version override: pass as $1 to test an arbitrary tag; defaults to the
# alef-pinned version from `[crates.e2e.registry.packages.php].version`.
VERSION="${1:-1.4.0-rc.46}"

# PIE >= 1.3.7 supports the array-form `php-ext.download-url-method`
# our composer.json emits; 1.4.0+ is preferred. Download PIE if we don't
# already have a recent enough version.
need_pie_install=true
if command -v pie >/dev/null 2>&1; then
  current="$(pie --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo '0.0.0')"
  if printf '%s\n%s\n' "1.3.7" "$current" | sort -V -C; then
    need_pie_install=false
  fi
fi
if [[ "$need_pie_install" == "true" ]]; then
  # Download PIE PHAR from latest GitHub release if not already installed.
  pie_dir="${HOME}/.local/bin"
  mkdir -p "$pie_dir"
  curl -fL --output "$pie_dir/pie" "https://github.com/php/pie/releases/latest/download/pie.phar" 2>/dev/null || {
    echo "::error::Failed to download PIE from GitHub; ensure network access or pre-install PIE." >&2
    exit 1
  }
  chmod +x "$pie_dir/pie"
  PIE="$pie_dir/pie"
  # Ensure newly downloaded PIE is on PATH for this script.
  export PATH="$pie_dir:$PATH"
else
  PIE="pie"
fi

# Install the extension binary into the running PHP's extension dir.
"$PIE" install "kreuzberg/liter-llm:$VERSION" --skip-enable-extension

# Verify the .so loads.
EXT_DIR="$(php -r 'echo ini_get("extension_dir");')"
test -f "$EXT_DIR/liter_llm.so" || test -f "$EXT_DIR/liter_llm.dylib" || test -f "$EXT_DIR/liter_llm.dll"

# Load it explicitly for the smoke test (the verify-install action runs
# phpunit with this same `-dextension=` flag in CI).
if ! php -dextension=liter_llm -m | grep -qi liter_llm; then
  echo "::error::liter_llm extension failed to load after PIE install" >&2
  exit 1
fi
echo "liter_llm extension installed and loaded"
