#!/usr/bin/env python3
"""Keep the workflows' ai-rulez installs on the latest release.

    workflows   every ai-rulez-version input in .github/workflows is "latest"

mise.toml pins the ai-rulez that tasks and hooks run, so no floor is checked here.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_INPUT = re.compile(r'^\s*ai-rulez-version:\s*"?([^"\s#]+)"?\s*(?:#.*)?$', re.MULTILINE)


def workflow_pins(workflows: Path = ROOT / ".github/workflows") -> dict[str, list[str]]:
    """Map each workflow file to the ai-rulez-version inputs it sets."""
    pins: dict[str, list[str]] = {}
    for path in sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")]):
        values = WORKFLOW_INPUT.findall(path.read_text())
        if values:
            pins[path.name] = values
    return pins


def check_workflows(pins: dict[str, list[str]]) -> None:
    """Require at least one workflow input and every one set to latest."""
    if not pins:
        raise SystemExit("No ai-rulez-version input found in the workflows")
    wrong = [f"{name}: {value}" for name, values in pins.items() for value in values if value != "latest"]
    if wrong:
        raise SystemExit("Workflow ai-rulez-version must be latest: " + "; ".join(wrong))


def main(argv: list[str]) -> None:
    """Run the named mode."""
    if len(argv) != 2 or argv[1] != "workflows":
        raise SystemExit("usage: check_ai_rulez_version.py workflows")
    check_workflows(workflow_pins())


if __name__ == "__main__":
    main(sys.argv)
