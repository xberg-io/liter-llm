#!/usr/bin/env bash
set -euo pipefail
# ~keep fetch, fetch_stdout and sha256 are defined per available tool and called below.
# shellcheck disable=SC2329

REPO="xberg-io/liter-llm"
# ~keep Rewritten by scripts/sync_plugin_version.py on every release. The plugin only accepts a
# ~keep binary at or above this version, and downloads exactly this release, so a plugin update
# ~keep cannot keep running an older liter-llm that happens to be installed.
PINNED_VERSION="2.7.6"

log() { printf 'liter-llm-launch: %s\n' "$*" >&2; }
die() {
  log "error: $*"
  exit 1
}

LAUNCHER="${LITER_LLM_LAUNCHER:-auto}"
case "$LAUNCHER" in
auto | brew | download) ;;
*) die "invalid LITER_LLM_LAUNCHER='$LAUNCHER' (expected auto|brew|download)" ;;
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

# True when $1 runs and reports a version at or above PINNED_VERSION. A prerelease of a version
# sorts below that version (SemVer), so 2.7.3-rc.1 does not satisfy a 2.7.3 pin. ~keep
runs_ok() {
  local out found base
  [ -x "$1" ] || return 1
  out="$("$1" --version 2>/dev/null)" || return 1
  found="$(printf '%s\n' "$out" | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?' | head -n 1)"
  [ -n "$found" ] || return 1
  base="${found%%-*}"
  if [ "$(printf '%s\n%s\n' "$PINNED_VERSION" "$base" | sort -V | head -n 1)" != "$PINNED_VERSION" ] ||
    { [ "$base" = "$PINNED_VERSION" ] && [ "$found" != "$base" ]; }; then
    log "ignoring $1: version $found is older than the pinned $PINNED_VERSION"
    return 1
  fi
}

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

# shellcheck disable=SC2329 # selected at runtime by the branch below ~keep
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

  local tag version
  version="$PINNED_VERSION"
  tag="v${version}"

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

  log "downloading $asset from release $tag ..."
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
or download a prebuilt archive from https://github.com/${REPO}/releases/tag/v${PINNED_VERSION}
then re-run, or set LITER_LLM_LAUNCHER to force a method (auto|brew|download)."
