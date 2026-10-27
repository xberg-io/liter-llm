#!/usr/bin/env python3
"""Keep ai-rulez on the latest release, with a floor for the binary the hooks run.

Two modes:

    installed   the ai-rulez on PATH is at least the Taskfile's AI_RULEZ_MIN_VERSION
    workflows   every ai-rulez-version input in .github/workflows is "latest"
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MINIMUM = re.compile(r'^\s*AI_RULEZ_MIN_VERSION:\s*"(\d+\.\d+\.\d+)"\s*$', re.MULTILINE)
WORKFLOW_INPUT = re.compile(r'^\s*ai-rulez-version:\s*"?([^"\s#]+)"?\s*(?:#.*)?$', re.MULTILINE)
REPORTED = re.compile(r"(?:ai-rulez(?: version)?\s+)?v?(\d+)\.(\d+)\.(\d+)")
UPGRADE = "uv tool install --upgrade ai-rulez"

Version = tuple[int, int, int]


def minimum_version(taskfile: Path = ROOT / "Taskfile.yml") -> Version:
    """Return the minimum release the Taskfile sets."""
    found = MINIMUM.findall(taskfile.read_text())
    if len(found) != 1:
        raise SystemExit(f"Expected one numeric AI_RULEZ_MIN_VERSION in {taskfile}, found {len(found)}")
    major, minor, patch = (int(part) for part in found[0].split("."))
    return (major, minor, patch)


def reported_version(output: str) -> Version:
    """Parse the whole `ai-rulez --version` output; a partial or pre-release match is a failure."""
    match = REPORTED.fullmatch(output.strip())
    if match is None:
        raise SystemExit(f"Cannot read an ai-rulez release version from: {output.strip()!r}")
    major, minor, patch = (int(part) for part in match.groups())
    return (major, minor, patch)


def workflow_pins(workflows: Path = ROOT / ".github/workflows") -> dict[str, list[str]]:
    """Map each workflow file to the ai-rulez-version inputs it sets."""
    pins: dict[str, list[str]] = {}
    for path in sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")]):
        values = WORKFLOW_INPUT.findall(path.read_text())
        if values:
            pins[path.name] = values
    return pins


def dotted(version: Version) -> str:
    """Render a version tuple as major.minor.patch."""
    return ".".join(str(part) for part in version)


def check_installed(minimum: Version) -> None:
    """Require the ai-rulez on PATH to be at least the minimum."""
    if shutil.which("ai-rulez") is None:
        raise SystemExit(f"Install ai-rulez {dotted(minimum)} or newer before installing hooks: {UPGRADE}")
    output = subprocess.run(["ai-rulez", "--version"], capture_output=True, text=True, check=True).stdout
    found = reported_version(output)
    if found < minimum:
        raise SystemExit(f"Hooks need ai-rulez {dotted(minimum)} or newer, found {dotted(found)}; run {UPGRADE}")


def check_workflows(pins: dict[str, list[str]]) -> None:
    """Require at least one workflow input and every one set to latest."""
    if not pins:
        raise SystemExit("No ai-rulez-version input found in the workflows")
    wrong = [f"{name}: {value}" for name, values in pins.items() for value in values if value != "latest"]
    if wrong:
        raise SystemExit("Workflow ai-rulez-version must be latest: " + "; ".join(wrong))


def main(argv: list[str]) -> None:
    """Run the named mode."""
    if len(argv) != 2 or argv[1] not in {"installed", "workflows"}:
        raise SystemExit("usage: check_ai_rulez_version.py installed|workflows")
    if argv[1] == "installed":
        check_installed(minimum_version())
    else:
        check_workflows(workflow_pins())


if __name__ == "__main__":
    main(sys.argv)
