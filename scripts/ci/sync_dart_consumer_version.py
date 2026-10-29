"""Synchronize the user-owned Dart registry consumer with the release version."""

import re
import sys
from pathlib import Path

import tomllib

REQUIREMENT = re.compile(r"(?m)^  liter_llm:[^\n]*$")


def synchronize(content: str, version: str) -> str:
    """Replace exactly one direct package requirement, preserving other dependencies."""
    updated, count = REQUIREMENT.subn(f"  liter_llm: {version}", content)
    if count != 1:
        raise SystemExit(f"Dart consumer must declare exactly one liter_llm requirement; found {count}")
    return updated


def main() -> None:
    """Apply the canonical pin, or reject drift when invoked with --check."""
    root = Path(__file__).resolve().parents[2]
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    version = manifest["workspace"]["package"]["version"]
    path = root / "test_apps/dart/pubspec.yaml"
    original = path.read_text()
    updated = synchronize(original, version)
    if original == updated:
        return
    if "--check" in sys.argv:
        raise SystemExit(f"Dart consumer must require liter_llm {version}; run mise run version:sync")
    path.write_text(updated)


if __name__ == "__main__":
    main()
