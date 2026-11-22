#!/usr/bin/env python3
"""Keep the CLI proxy packages and the MCP Registry entry in lockstep with the release.

``alef sync-versions`` rewrites ``cli-proxy/npm/package.json`` and ``cli-proxy/pypi/pyproject.toml``
(``[workspace.sync].extra_paths``) but knows nothing about the proxy's embedded ``__version__``
or ``server.json``, and its own validator does not read extra paths, so nothing else would fail on
drift. This script owns those two files and verifies all five.

Usage:
    python3 scripts/ci/check_cli_proxy_versions.py                # verify against Cargo.toml
    python3 scripts/ci/check_cli_proxy_versions.py --sync         # rewrite what alef does not own, then verify
    python3 scripts/ci/check_cli_proxy_versions.py --expect X.Y.Z # verify against the release being published
"""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
NPM_MANIFEST = ROOT / "cli-proxy" / "npm" / "package.json"
PYPI_MANIFEST = ROOT / "cli-proxy" / "pypi" / "pyproject.toml"
PYPI_INIT = ROOT / "cli-proxy" / "pypi" / "liter_llm_cli" / "__init__.py"
SERVER_JSON = ROOT / "server.json"
NPM_PACKAGE = "@xberg-io/liter-llm-cli"
SERVER_NAME = "io.github.xberg-io/liter-llm"
INIT_VERSION_RE = re.compile(r'(?m)^(__version__ = ")([^"]*)(")')
SYNC_HINT = "run `mise run version:sync`"


def core_version(root: Path = ROOT) -> str:
    """Read the workspace crate version, the repository's version source of truth."""
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    return str(manifest["workspace"]["package"]["version"])


def read_versions(root: Path = ROOT) -> dict[str, str]:
    """Return every version this script is responsible for, keyed by where it lives."""
    npm = json.loads((root / "cli-proxy/npm/package.json").read_text(encoding="utf-8"))
    pypi = tomllib.loads((root / "cli-proxy/pypi/pyproject.toml").read_text(encoding="utf-8"))
    init = INIT_VERSION_RE.search((root / "cli-proxy/pypi/liter_llm_cli/__init__.py").read_text(encoding="utf-8"))
    server = json.loads((root / "server.json").read_text(encoding="utf-8"))
    packages = [p for p in server.get("packages", []) if p.get("identifier") == NPM_PACKAGE]
    return {
        "cli-proxy/npm/package.json version": str(npm.get("version")),
        "cli-proxy/pypi/pyproject.toml version": str(pypi["project"]["version"]),
        "cli-proxy/pypi/liter_llm_cli/__init__.py __version__": init.group(2) if init else "<missing>",
        "server.json version": str(server.get("version")),
        "server.json packages[npm].version": str(packages[0].get("version")) if len(packages) == 1 else "<missing>",
    }


def read_identity(root: Path = ROOT) -> list[str]:
    """Return the ownership-proof mismatches between package.json and server.json."""
    npm = json.loads((root / "cli-proxy/npm/package.json").read_text(encoding="utf-8"))
    server = json.loads((root / "server.json").read_text(encoding="utf-8"))
    problems = []
    if server.get("name") != SERVER_NAME:
        problems.append(f"server.json name is {server.get('name')!r}, expected {SERVER_NAME!r}")
    if npm.get("mcpName") != server.get("name"):
        problems.append(
            f"cli-proxy/npm/package.json mcpName {npm.get('mcpName')!r} must equal server.json name "
            f"{server.get('name')!r}; the MCP Registry rejects the publish otherwise"
        )
    if npm.get("name") != NPM_PACKAGE:
        problems.append(f"cli-proxy/npm/package.json name is {npm.get('name')!r}, expected {NPM_PACKAGE!r}")
    return problems


def sync(version: str, root: Path = ROOT) -> None:
    """Rewrite the two files alef does not manage."""
    init_path = root / "cli-proxy/pypi/liter_llm_cli/__init__.py"
    text = init_path.read_text(encoding="utf-8")
    text, count = INIT_VERSION_RE.subn(rf"\g<1>{version}\g<3>", text, count=1)
    if count != 1:
        sys.exit(f"could not find __version__ in {init_path.relative_to(root)}")
    init_path.write_text(text, encoding="utf-8")

    server_path = root / "server.json"
    server = json.loads(server_path.read_text(encoding="utf-8"))
    server["version"] = version
    packages = [p for p in server.get("packages", []) if p.get("identifier") == NPM_PACKAGE]
    if len(packages) != 1:
        sys.exit(f"server.json must list exactly one {NPM_PACKAGE} package, found {len(packages)}")
    packages[0]["version"] = version
    server_path.write_text(json.dumps(server, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def drift(expected: str, root: Path = ROOT) -> list[str]:
    """Return one line per version that is not ``expected`` or per broken identity link."""
    lines = [f"  {where} -> {found}" for where, found in read_versions(root).items() if found != expected]
    return lines + [f"  {problem}" for problem in read_identity(root)]


def parse_expect(argv: list[str]) -> str | None:
    """Return the version passed to ``--expect``, or ``None`` when absent."""
    for index, arg in enumerate(argv):
        if arg == "--expect":
            if index + 1 >= len(argv):
                sys.exit("--expect requires a version argument")
            return argv[index + 1]
        if arg.startswith("--expect="):
            return arg.split("=", 1)[1]
    return None


def main() -> int:
    argv = sys.argv[1:]
    expected = parse_expect(argv) or core_version()
    if "--sync" in argv:
        sync(expected)
    problems = drift(expected)
    if problems:
        print(f"CLI proxy / MCP registry drift: expected {expected}")
        print("\n".join(problems))
        print(SYNC_HINT)
        return 1
    print(f"CLI proxy packages and server.json all at {expected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
