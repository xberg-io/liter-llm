#!/usr/bin/env python3
"""Keep every ai-rulez version pin equal to the Taskfile's AI_RULEZ_VERSION.

Two modes:

    installed   the ai-rulez on PATH reports exactly the pinned version
    workflows   every ai-rulez-version input in .github/workflows equals the pin
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PIN = re.compile(r'^\s*AI_RULEZ_VERSION:\s*"([^"]+)"\s*$', re.MULTILINE)
WORKFLOW_INPUT = re.compile(r'^\s*ai-rulez-version:\s*"?([^"\s#]+)"?\s*(?:#.*)?$', re.MULTILINE)
REPORTED = re.compile(r"(?:ai-rulez(?: version)?\s+)?v?(\d+\.\d+\.\d+)")


def pinned_version(taskfile: Path = ROOT / "Taskfile.yml") -> str:
    """Return the version the Taskfile pins."""
    found = PIN.findall(taskfile.read_text())
    if len(found) != 1:
        raise SystemExit(f"Expected one AI_RULEZ_VERSION in {taskfile}, found {len(found)}")
    return found[0]


def reported_version(output: str) -> str:
    """Parse the whole `ai-rulez --version` output; a partial match is a failure."""
    match = REPORTED.fullmatch(output.strip())
    if match is None:
        raise SystemExit(f"Cannot read an ai-rulez version from: {output.strip()!r}")
    return match.group(1)


def workflow_pins(workflows: Path = ROOT / ".github/workflows") -> dict[str, list[str]]:
    """Map each workflow file to the ai-rulez-version inputs it sets."""
    pins: dict[str, list[str]] = {}
    for path in sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")]):
        values = WORKFLOW_INPUT.findall(path.read_text())
        if values:
            pins[path.name] = values
    return pins


def check_installed(pin: str) -> None:
    """Require the ai-rulez on PATH to equal the pin."""
    if shutil.which("ai-rulez") is None:
        raise SystemExit(f"Install ai-rulez {pin} before installing hooks")
    output = subprocess.run(["ai-rulez", "--version"], capture_output=True, text=True, check=True).stdout
    found = reported_version(output)
    if found != pin:
        raise SystemExit(f"Hooks need ai-rulez {pin}, found: {found}")


def check_workflows(pin: str, pins: dict[str, list[str]]) -> None:
    """Require at least one workflow pin and every one equal to the Taskfile pin."""
    if not pins:
        raise SystemExit("No ai-rulez-version input found in the workflows")
    wrong = [f"{name}: {value}" for name, values in pins.items() for value in values if value != pin]
    if wrong:
        raise SystemExit(f"Workflow ai-rulez-version differs from Taskfile AI_RULEZ_VERSION {pin}: " + "; ".join(wrong))


def main(argv: list[str]) -> None:
    """Run the named mode."""
    if len(argv) != 2 or argv[1] not in {"installed", "workflows"}:
        raise SystemExit("usage: check_ai_rulez_version.py installed|workflows")
    pin = pinned_version()
    if argv[1] == "installed":
        check_installed(pin)
    else:
        check_workflows(pin, workflow_pins())


if __name__ == "__main__":
    main(sys.argv)
