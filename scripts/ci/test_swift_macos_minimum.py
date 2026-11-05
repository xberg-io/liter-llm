"""Every Swift manifest and the install guide declare the macOS minimum of the root Package.swift.

The Swift package requires the lowest macOS that its generated code needs: macOS 13, because alef 0.124.2 emits
Swift's Duration, which needs macOS 13. The root Package.swift is the one source of truth. alef.toml sets the same
value, and alef emits it into the e2e and test_apps packages. The root and in-tree manifests are alef create-once
files that keep whatever was first written, so a hand edit to one of them splits the floor and SwiftPM refuses the
e2e package that depends on the product. The Rust binaries have their own, lower deployment target, which
test_macos_deployment_target.py guards.
"""

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFESTS = (
    "Package.swift",
    "packages/swift/Package.swift",
    "e2e/swift_e2e/Package.swift",
    "test_apps/swift_e2e/Package.swift",
)


def declared_minimum(path: str) -> list[str]:
    return re.findall(r"\.macOS\(\.v(\d+)\)", (ROOT / path).read_text())


def package_minimum() -> str:
    (major,) = declared_minimum("Package.swift")
    return major


def test_the_package_minimum_is_macos_13() -> None:
    assert package_minimum() == "13"


def test_every_swift_manifest_declares_the_package_minimum() -> None:
    assert {path: declared_minimum(path) for path in MANIFESTS} == {path: [package_minimum()] for path in MANIFESTS}


def test_alef_config_sets_the_package_minimum() -> None:
    with (ROOT / "alef.toml").open("rb") as config:
        (swift,) = [crate["swift"] for crate in tomllib.load(config)["crates"] if "swift" in crate]
    assert swift["min_macos_version"] == f"{package_minimum()}.0"


def test_install_guide_states_the_package_minimum() -> None:
    guide = (ROOT / "docs-site/src/content/docs/getting-started/installation.mdx").read_text()
    assert re.findall(r"Requires macOS (\d+)\+", guide) == [package_minimum()]
