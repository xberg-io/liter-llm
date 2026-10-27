"""Check maintained registry coordinates without resolving unpublished packages."""

import re
import sys
from pathlib import Path

import tomllib
from check_kotlin_consumer_version import synchronize_coordinates, verify_coordinates
from sync_dart_consumer_version import synchronize

# Locks are deliberately excluded: they can only advance after registry publication.
PINS = {
    "node/package.json": r'"@xberg-io/liter-llm"\s*:\s*"([^"]+)"',
    "wasm/package.json": r'"@xberg-io/liter-llm-wasm"\s*:\s*"([^"]+)"',
    "python/pyproject.toml": r'"liter-llm==([^"]+)"',
    "ruby/Gemfile": r"(?m)^gem 'liter_llm', '([^']+)'",
    "csharp/LiterLlm.E2eTests.csproj": r'PackageReference Include="XbergIo.LiterLlm" Version="([^"]+)"',
    "java/pom.xml": r"<artifactId>liter-llm</artifactId>\s*<version>([^<]+)</version>",
    "go/go.mod": r"(?m)^\s*(github.com/xberg-io/liter-llm/packages/go(?:/v\d+)?[ \t]+v[^\s]+)",
    "rust/Cargo.toml": r'package = "liter-llm", version = "([^"]+)"',
    "elixir/mix.exs": r'(?m)^\s*\{:liter_llm, "([^"]+)"\}',
    "swift_e2e/Package.swift": r'branch: "release/swift/([^"]+)"',
    "c/download_ffi.sh": r"(?m)^VERSION='([^']+)'",
    "php/install.sh": r"(?m)^PINNED_VERSION='([^']+)'",
    "homebrew/homebrew.bats": r"(?m)^export VERSION='([^']+)'",
    "homebrew/README.md": r"at version `([^`]+)`",
    "zig/build.zig.zon": r'\.url = "([^"]+)"',
}
OWNED_PINS = (
    "node/package.json",
    "wasm/package.json",
    "csharp/LiterLlm.E2eTests.csproj",
    "java/pom.xml",
    "go/go.mod",
    "elixir/mix.exs",
    "homebrew/README.md",
    "zig/build.zig.zon",
)


def expected_pin(path: str, version: str) -> str:
    """Return the version or complete archive coordinate for this manifest."""
    if path == "go/go.mod":
        major = int(version.split(".", maxsplit=1)[0])
        suffix = f"/v{major}" if major >= 2 else ""
        return f"github.com/xberg-io/liter-llm/packages/go{suffix} v{version}"
    if path == "zig/build.zig.zon":
        return f"https://github.com/xberg-io/liter-llm/releases/download/v{version}/liter-llm-zig-v{version}.tar.gz"
    return version


def verify_pin(content: str, pattern: str, expected: str, path: str) -> None:
    """Reject stale, absent, or duplicate coordinates instead of passing zero work."""
    values = re.findall(pattern, content)
    if values != [expected]:
        raise SystemExit(f"Registry consumer {path} must contain exactly one {expected} pin; found {values}")


def replace_pin(content: str, pattern: str, version: str) -> str:
    """Change one user-owned coordinate without reformatting the surrounding file."""
    matches = list(re.finditer(pattern, content))
    if len(matches) != 1:
        raise SystemExit("Registry consumer must contain exactly one editable coordinate")
    match = matches[0]
    return content[: match.start(1)] + version + content[match.end(1) :]


def sync_owned(root: Path, version: str) -> None:
    """Update create-once manifests which Alef intentionally leaves user-owned."""
    for relative in OWNED_PINS:
        path = root / relative
        path.write_text(replace_pin(path.read_text(), PINS[relative], expected_pin(relative, version)))
    dart = root / "dart/pubspec.yaml"
    dart.write_text(synchronize(dart.read_text(), version))
    kotlin = root / "kotlin_android/build.gradle.kts"
    kotlin.write_text(synchronize_coordinates(kotlin.read_text(), version))


def verify_all(root: Path, version: str) -> None:
    """Verify every maintained package coordinate, including native-only installers."""
    for relative, pattern in PINS.items():
        verify_pin((root / relative).read_text(), pattern, expected_pin(relative, version), relative)
    dart = (root / "dart/pubspec.yaml").read_text()
    if synchronize(dart, version) != dart:
        raise SystemExit(f"Dart registry consumer must select {version}")
    verify_coordinates((root / "kotlin_android/build.gradle.kts").read_text(), version)


def main() -> None:
    """Synchronize owned pins when requested, then check all maintained consumers."""
    repository = Path(__file__).resolve().parents[2]
    manifest = tomllib.loads((repository / "Cargo.toml").read_text())
    version = manifest["workspace"]["package"]["version"]
    root = repository / "test_apps"
    if "--sync" in sys.argv:
        sync_owned(root, version)
    verify_all(root, version)
    print(f"All maintained registry consumer coordinates match {version}; locks are checked after publication")


if __name__ == "__main__":
    main()
