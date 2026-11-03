#!/usr/bin/env bash
set -euo pipefail

REPO="xberg-io/liter-llm"
NPM_PKG="@xberg-io/liter-llm-cli"
PYPI_PKG="liter-llm-cli"

log() { printf 'liter-llm-launch: %s\n' "$*" >&2; }
die() {
  log "error: $*"
  exit 1
}

LAUNCHER="${LITER_LLM_LAUNCHER:-auto}"
case "$LAUNCHER" in
auto | npx | uvx | brew | download) ;;
*) die "invalid LITER_LLM_LAUNCHER='$LAUNCHER' (expected auto|npx|uvx|brew|download)" ;;
esac

want() { [ "$LAUNCHER" = "auto" ] || [ "$LAUNCHER" = "$1" ]; }

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-}"
if [ -z "$PLUGIN_ROOT" ]; then
  PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

BINARY_NAME="liter-llm"
case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN* | Windows_NT) BINARY_NAME="liter-llm.exe" ;;
esac
BIN_DIR="$PLUGIN_ROOT/bin"
BIN="$BIN_DIR/$BINARY_NAME"

have() { command -v "$1" >/dev/null 2>&1; }

runs_ok() { [ -x "$1" ] && "$1" --version >/dev/null 2>&1; }

if runs_ok "$BIN"; then
  log "using cached liter-llm at $BIN"
  exec "$BIN" "$@"
fi
if have "$BINARY_NAME"; then
  PATH_BIN="$(command -v "$BINARY_NAME")"
  if runs_ok "$PATH_BIN"; then
    log "using liter-llm on PATH ($PATH_BIN)"
    exec "$PATH_BIN" "$@"
  fi
fi

if want npx && have npx; then
  log "resolving npx $NPM_PKG@latest ..."
  scratch="$(mktemp -d)"
  # Resolve in an empty project, then execute the selected binary in the caller's cwd. ~keep
  if npm_bin="$(cd "$scratch" && npx -y --package "$NPM_PKG@latest" --call "command -v ${BINARY_NAME%.exe}" 2>/dev/null)" && runs_ok "$npm_bin"; then
    rm -rf "$scratch"
    log "launching npm package binary $npm_bin"
    exec "$npm_bin" "$@"
  fi
  rm -rf "$scratch"
  log "npx $NPM_PKG not available (no CLI bin yet); falling through"
fi

if want uvx && have uvx; then
  log "probing uvx --from $PYPI_PKG $BINARY_NAME ..."
  if uvx --from "$PYPI_PKG" "$BINARY_NAME" --version >/dev/null 2>&1; then
    log "launching via uvx --from $PYPI_PKG $BINARY_NAME"
    exec uvx --from "$PYPI_PKG" "$BINARY_NAME" "$@"
  fi
  log "uvx $PYPI_PKG not available (no CLI entry point yet); falling through"
fi

if want brew && have brew; then
  log "installing via 'brew install xberg-io/tap/liter-llm' ..."
  if brew install xberg-io/tap/liter-llm >&2; then
    if have "$BINARY_NAME"; then
      BREW_BIN="$(command -v "$BINARY_NAME")"
      runs_ok "$BREW_BIN" && exec "$BREW_BIN" "$@"
    fi
    log "brew install reported success but liter-llm is not on PATH; falling through"
  else
    log "brew install failed; falling through"
  fi
fi

if have curl; then
  fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
  fetch_stdout() { curl -fsSL --retry 3 "$1"; }
elif have wget; then
  fetch() { wget -q -O "$2" "$1"; }
  fetch_stdout() { wget -q -O - "$1"; }
else
  fetch() { return 1; }
  fetch_stdout() { return 1; }
fi

if have sha256sum; then
  sha256() { sha256sum "$1" | awk '{print $1}'; }
elif have shasum; then
  sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  sha256() { return 1; }
fi

try_download() {
  if ! have curl && ! have wget; then
    log "download: no curl or wget available; falling through"
    return 1
  fi

  arch="$(uname -m)"
  local target ext
  case "$(uname -s)" in
  Darwin)
    case "$arch" in
    arm64 | aarch64) target="aarch64-apple-darwin" ;;
    *)
      log "download: Intel macOS (x86_64) prebuilt is not shipped; falling through"
      return 1
      ;;
    esac
    ext="tar.gz"
    ;;
  Linux)
    case "$arch" in
    aarch64 | arm64) target="aarch64-unknown-linux-gnu" ;;
    *) target="x86_64-unknown-linux-gnu" ;;
    esac
    ext="tar.gz"
    ;;
  MINGW* | MSYS* | CYGWIN* | Windows_NT)
    target="x86_64-pc-windows-msvc"
    ext="zip"
    ;;
  *)
    log "download: unsupported platform $(uname -s) $arch; falling through"
    return 1
    ;;
  esac

  local api_json tag version
  api_json="$(fetch_stdout "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null)" ||
  {
    log "download: could not query latest release; falling through"
    return 1
  }
  tag="$(printf '%s' "$api_json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$tag" ] || {
    log "download: could not parse tag_name from API; falling through"
    return 1
  }
  version="${tag#v}"

  local stem base_url asset asset_url sums_url
  stem="liter-llm-${version}-${target}"
  base_url="https://github.com/${REPO}/releases/download/${tag}"
  asset="${stem}.${ext}"
  asset_url="${base_url}/${asset}"
  sums_url="${base_url}/SHA256SUMS-${version}.txt"

  local tmp
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" RETURN

  log "downloading $asset from $tag ..."
  if ! fetch "$asset_url" "$tmp/$asset"; then
    log "download: asset not found ($asset_url); falling through"
    return 1
  fi

  if sha256 "$tmp/$asset" >/dev/null 2>&1 && fetch "$sums_url" "$tmp/checksums.txt" 2>/dev/null; then
    local expected actual
    expected="$(awk -v f="$asset" '{name=$NF; sub(/^[*]/, "", name); if (name == f) print $1}' "$tmp/checksums.txt")"
    if [ -n "$expected" ]; then
      actual="$(sha256 "$tmp/$asset")"
      [ "$expected" = "$actual" ] || die "checksum mismatch for $asset (expected $expected, got $actual)"
      log "checksum verified"
    else
      log "WARNING: no checksum entry for $asset in SHA256SUMS-${version}.txt; proceeding over HTTPS unverified"
    fi
  else
    log "WARNING: could not verify checksum (no sha256 tool or no SHA256SUMS file); proceeding over HTTPS unverified"
  fi

  local ex src_bin
  ex="$tmp/extracted"
  mkdir -p "$ex"
  case "$ext" in
  tar.gz) tar -xzf "$tmp/$asset" -C "$ex" ;;
  zip)
    have unzip || {
      log "download: need unzip to extract $asset; falling through"
      return 1
    }
    unzip -qo "$tmp/$asset" -d "$ex"
    ;;
  esac
  src_bin="$ex/$stem/$BINARY_NAME"
  [ -f "$src_bin" ] || {
    log "download: $BINARY_NAME not found inside $asset; falling through"
    return 1
  }

  rm -rf "$BIN_DIR"
  mkdir -p "$BIN_DIR"
  mv "$src_bin" "$BIN"
  chmod +x "$BIN"
  log "installed liter-llm $version to $BIN"
  runs_ok "$BIN" || {
    log "download: installed binary failed to run; falling through"
    return 1
  }
  return 0
}

if want download; then
  if try_download; then
    exec "$BIN" "$@"
  fi
fi

die "could not locate or install liter-llm. Install it with one of:
  brew install xberg-io/tap/liter-llm
  cargo install liter-llm-cli
or download a prebuilt archive from https://github.com/${REPO}/releases/latest
then re-run, or set LITER_LLM_LAUNCHER to force a method (auto|npx|uvx|brew|download)."
