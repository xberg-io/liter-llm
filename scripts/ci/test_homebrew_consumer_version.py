"""Check the Homebrew version assertion uses the current release value."""

import os
import subprocess
import tomllib
from pathlib import Path

import pytest


@pytest.mark.parametrize(("reported", "matches"), [("2.0.0", True), ("1.0.0", False)])
def test_version_command_rejects_another_installed_release(tmp_path: Path, reported: str, matches: bool) -> None:
    """The generated harness matches a literal marker only after exact version validation."""
    root = Path(__file__).resolve().parents[2]
    config = tomllib.loads((root / "alef.toml").read_text())
    probe = config["crates"][0]["e2e"]["registry"]["packages"]["homebrew"]["cli_tests"][0]
    command = tmp_path / "liter-llm"
    command.write_text(f'#!/bin/sh\nprintf "%s\\n" "liter-llm {reported}"\n')
    command.chmod(0o755)
    environment = {
        **os.environ,
        "PATH": f"{tmp_path}:{os.environ['PATH']}",
        "CLI_FORMULA": "liter-llm",
        "VERSION": "2.0.0",
    }
    result = subprocess.run(
        ["bash", "-c", probe["command"]], env=environment, capture_output=True, text=True, check=False
    )
    assert (probe["expect_contains"] in result.stdout) is matches
    assert (result.returncode == 0) is matches
