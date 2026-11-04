"""Every Swift manifest and the install guide declare the same macOS minimum.

The org floor is macOS 26 and alef emits it into the e2e and test_apps packages. The root and in-tree
manifests are alef create-once files that keep whatever was first written, so a hand edit to one of them
splits the floor and SwiftPM refuses the e2e package that depends on the product.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MACOS_MINIMUM = "26"
MANIFESTS = (
    "Package.swift",
    "packages/swift/Package.swift",
    "e2e/swift_e2e/Package.swift",
    "test_apps/swift_e2e/Package.swift",
)


def test_every_swift_manifest_declares_the_org_macos_minimum() -> None:
    declared = {path: re.findall(r"\.macOS\(\.v(\d+)\)", (ROOT / path).read_text()) for path in MANIFESTS}
    assert declared == {path: [MACOS_MINIMUM] for path in MANIFESTS}


def test_install_guide_states_the_macos_minimum() -> None:
    guide = (ROOT / "docs-site/src/content/docs/getting-started/installation.mdx").read_text()
    assert re.findall(r"Requires macOS (\d+)\+", guide) == [MACOS_MINIMUM]
