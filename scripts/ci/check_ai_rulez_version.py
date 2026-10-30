#!/usr/bin/env python3
"""Keep ai-rulez unpinned (org ADR-0015).

workflows   every ai-rulez-version input in .github/workflows is "latest", and no setup-mise step
            installs github:Goldziher/ai-rulez
mise        no mise*.toml pins github:Goldziher/ai-rulez
hooks       the ai-rulez hook source in alef.toml and poly.toml follows the floating v5 tag
all         every check above
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_INPUT = re.compile(r'^\s*ai-rulez-version:\s*"?([^"\s#]+)"?\s*(?:#.*)?$', re.MULTILINE)
MISE_INSTALL = re.compile(r"^\s*install-args:.*github:Goldziher/ai-rulez", re.MULTILINE)
MISE_PIN = re.compile(r'^\s*"github:Goldziher/ai-rulez"\s*=', re.MULTILINE)
HOOK_REVISION = "v5"


def workflow_pins(workflows: Path = ROOT / ".github/workflows") -> dict[str, list[str]]:
    """Map each workflow file to the ai-rulez-version inputs it sets."""
    pins: dict[str, list[str]] = {}
    for path in sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")]):
        values = WORKFLOW_INPUT.findall(path.read_text())
        if values:
            pins[path.name] = values
    return pins


def mise_installs(workflows: Path = ROOT / ".github/workflows") -> list[str]:
    """Name the workflow files that install ai-rulez through setup-mise."""
    paths = sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")])
    return [path.name for path in paths if MISE_INSTALL.search(path.read_text())]


def check_workflows(pins: dict[str, list[str]], installs: list[str] | None = None) -> None:
    """Require at least one workflow input, every one set to latest, and no setup-mise install."""
    if not pins:
        raise SystemExit("No ai-rulez-version input found in the workflows")
    wrong = [f"{name}: {value}" for name, values in pins.items() for value in values if value != "latest"]
    if wrong:
        raise SystemExit("Workflow ai-rulez-version must be latest: " + "; ".join(wrong))
    if installs:
        raise SystemExit(
            "Install ai-rulez with xberg-io/actions/setup-ai-rulez, not setup-mise: " + ", ".join(installs)
        )


def check_mise(root: Path = ROOT) -> None:
    """Refuse an ai-rulez entry in any mise config."""
    configs = sorted(root.glob("mise*.toml"))
    if not configs:
        raise SystemExit(f"No mise*.toml found in {root}")
    pinned = [path.name for path in configs if MISE_PIN.search(path.read_text())]
    if pinned:
        raise SystemExit("ai-rulez is never pinned; remove it from: " + ", ".join(pinned))


def hook_revision(path: Path, prefix: tuple[str, ...]) -> str | None:
    """Return the revision of the ai-rulez hook source in a TOML file, under the given table path."""
    table = tomllib.loads(path.read_text())
    for key in prefix:
        table = table.get(key, {})
    found = [source for source in table if source.get("id") == "ai-rulez"] if isinstance(table, list) else []
    if len(found) != 1:
        raise SystemExit(f"{path.name}: expected exactly one ai-rulez hook source, found {len(found)}")
    return found[0].get("revision")


def check_hooks(root: Path = ROOT) -> None:
    """Require the ai-rulez hook source on the floating major tag in alef.toml and the poly.toml it renders."""
    revisions = {
        "alef.toml": hook_revision(root / "alef.toml", ("workspace", "poly", "hooks-sources")),
        "poly.toml": hook_revision(root / "poly.toml", ("hooks", "sources")),
    }
    wrong = [f"{name}: {revision}" for name, revision in revisions.items() if revision != HOOK_REVISION]
    if wrong:
        raise SystemExit(f"ai-rulez hook source must follow {HOOK_REVISION}: " + "; ".join(wrong))


def main(argv: list[str]) -> None:
    """Run the named mode."""
    modes = {
        "workflows": lambda: check_workflows(workflow_pins(), mise_installs()),
        "mise": check_mise,
        "hooks": check_hooks,
    }
    if len(argv) != 2 or argv[1] not in [*modes, "all"]:
        raise SystemExit("usage: check_ai_rulez_version.py workflows|mise|hooks|all")
    for name, run in modes.items():
        if argv[1] in (name, "all"):
            run()


if __name__ == "__main__":
    main(sys.argv)
