"""Ensure every native Node build preserves Alef's public loader and declarations."""

import json
import shlex
from pathlib import Path

import pytest
import tomllib
import yaml

ROOT = Path(__file__).resolve().parents[2]


def build_commands() -> dict[str, str]:
    """Read executable commands rather than matching comments or unused examples."""
    package = json.loads((ROOT / "crates/liter-llm-node/package.json").read_text())
    with (ROOT / "mise-tasks/languages.toml").open("rb") as config:
        tasks = tomllib.load(config)
    workflow = yaml.safe_load((ROOT / ".github/workflows/publish.yaml").read_text())
    native_steps = [
        step
        for job in workflow["jobs"].values()
        for step in job.get("steps", [])
        if step.get("name") == "Build NAPI binding"
    ]
    assert len(native_steps) == 1, "Expected exactly one native Node publish build"
    return {
        "package": package["scripts"]["build"],
        "task-release": tasks["node:build"]["run"],
        "task-debug": tasks["node:build:dev"]["run"],
        "publish": native_steps[0]["run"],
    }


@pytest.mark.parametrize("path", ["package", "task-release", "task-debug", "publish"])
def test_native_build_preserves_public_files(path: str) -> None:
    """Native metadata must go to its private artifact, never the public TypeScript API."""
    tokens = shlex.split(build_commands()[path], comments=True)
    assert "napi" in tokens, path
    assert tokens[tokens.index("napi") + 1] == "build", path
    assert "--no-js" in tokens, f"{path} would overwrite index.js"
    assert tokens.count("--dts") == 1, f"{path} must redirect native declarations"
    assert tokens[tokens.index("--dts") + 1] == "index.native.d.ts", path
    assert "--platform" in tokens, f"{path} would write a .node file the index.js loader never looks for"
