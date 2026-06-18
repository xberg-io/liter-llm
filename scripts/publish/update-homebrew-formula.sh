#!/usr/bin/env bash
set -euo pipefail

# Update Formula/liter-llm.rb in the homebrew-tap with the new tag's URL and
# source-tarball SHA256. The bottle DSL is updated separately by the
# `homebrew-merge-bottles@v1` action after bottles are built.
#
# Usage (env vars):
#   TAG=v1.4.0-rc.31 VERSION=1.4.0-rc.31 \
#   TAP_DIR=/path/to/homebrew-tap \
#   ./update-homebrew-formula.sh

tag="${TAG:?TAG is required (e.g. v1.4.0-rc.31)}"
version="${VERSION:?VERSION is required (e.g. 1.4.0-rc.31)}"
tap_dir="${TAP_DIR:?TAP_DIR is required (path to homebrew-tap checkout)}"
dry_run="${DRY_RUN:-false}"

formula="${tap_dir}/Formula/liter-llm.rb"

[[ -f "$formula" ]] || {
  echo "Missing $formula" >&2
  exit 1
}

tarball_url="https://github.com/kreuzberg-dev/liter-llm/archive/${tag}.tar.gz"

echo "Updating Homebrew formula for liter-llm ${version} (tag ${tag})"

if [[ "$dry_run" == "true" ]]; then
  echo "[dry-run] target formula: $formula"
  echo "[dry-run] would set url to: $tarball_url"
  echo "[dry-run] would compute sha256 of source tarball and rewrite the formula"
  echo "[dry-run] would leave bottle DSL untouched (handled by homebrew-merge-bottles)"
  exit 0
fi

echo "Fetching source tarball SHA256 for ${tag}..."
sha256=$(curl -fsSL "$tarball_url" | shasum -a 256 | awk '{print $1}')
echo "  url:    $tarball_url"
echo "  sha256: $sha256"

# Update the top-level url + sha256 lines (the ones outside `bottle do ... end`).
# Match `url "..."` on one line, `sha256 "..."` on the next, only when both come
# before the `bottle do` block.
python3 - "$formula" "$tarball_url" "$sha256" <<'PY'
import re
import sys

formula_path, new_url, new_sha = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(formula_path).read()

# Split off the bottle block so the regex only touches the formula header.
bottle_start = text.find("bottle do")
if bottle_start == -1:
    head, tail = text, ""
else:
    head, tail = text[:bottle_start], text[bottle_start:]

head = re.sub(r"""^(\s*url\s+)["'][^"']*["']""", rf'\1"{new_url}"', head, count=1, flags=re.MULTILINE)
head = re.sub(r"""^(\s*sha256\s+)["'][^"']*["']""", rf'\1"{new_sha}"', head, count=1, flags=re.MULTILINE)

# liter-llm-cli pulls liter-llm-proxy -> etcd-client v0.15 (with the
# `etcd-watch` feature) whose build.rs shells out to `protoc` via prost-build.
# `etcd-watch` is off by default since v1.6.4, so this dep is only a no-op
# safety net on default brew builds — but it does no harm to keep it. Idempotent.
if "depends_on 'protobuf' => :build" not in head and 'depends_on "protobuf" => :build' not in head:
    head = re.sub(
        r"""(^\s*depends_on\s+['"]rust['"]\s+=>\s+:build[^\n]*\n)""",
        r"\1  depends_on 'protobuf' => :build\n",
        head,
        count=1,
        flags=re.MULTILINE,
    )

# opendal-core (used by liter-llm-proxy's opendal-cache feature) unconditionally
# pulls reqwest with `hyper-tls` -> `native-tls` -> `openssl-sys`. brew's
# arm64_linux/x86_64_linux source-build sandbox lacks system OpenSSL, so the
# `openssl-sys` build script fails with "Could not find directory of OpenSSL
# installation". Without an upstream fix in opendal-core to honour `default-
# features = false` on its reqwest dep, brew needs `openssl@3` as a build dep.
# Idempotent injection.
if "depends_on 'openssl@3' => :build" not in head and 'depends_on "openssl@3" => :build' not in head:
    head = re.sub(
        r"""(^\s*depends_on\s+['"]protobuf['"]\s+=>\s+:build[^\n]*\n)""",
        r"\1  depends_on 'openssl@3' => :build\n",
        head,
        count=1,
        flags=re.MULTILINE,
    )

with open(formula_path, "w") as f:
    f.write(head + tail)
PY

echo "Updated $formula"
