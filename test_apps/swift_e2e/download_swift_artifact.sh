#!/bin/bash
set -euo pipefail

# Download the Swift artifact bundle and compute its checksum.
# SwiftPM requires a stable SHA256 checksum for binary targets.

ARTIFACT_URL="https://github.com/kreuzberg-dev/liter-llm/releases/download/v1.4.0-rc.48/LiterLlm-rs.artifactbundle.zip"
ARTIFACT_FILE="LiterLlm-rs.artifactbundle.zip"
PACKAGE_SWIFT="Package.swift"

# Download the artifact if not already cached
if [ ! -f "$ARTIFACT_FILE" ]; then
  echo "Downloading Swift artifact from $ARTIFACT_URL"
  curl -fsSL -o "$ARTIFACT_FILE" "$ARTIFACT_URL"
else
  echo "Using cached artifact: $ARTIFACT_FILE"
fi

# Compute SHA256 checksum
CHECKSUM=$(swift package compute-checksum "$ARTIFACT_FILE")
echo "Computed checksum: $CHECKSUM"

# Substitute the placeholder checksum in Package.swift
sed -i.bak "s/__ALEF_SWIFT_CHECKSUM__/$CHECKSUM/g" "$PACKAGE_SWIFT"
rm -f "${PACKAGE_SWIFT}.bak"

echo "Updated $PACKAGE_SWIFT with checksum"
