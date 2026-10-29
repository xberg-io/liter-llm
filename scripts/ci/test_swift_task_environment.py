"""Keep Apple deployment settings confined to Swift build tasks."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest
import tomllib

ROOT = Path(__file__).resolve().parents[2]
APPLE_TARGETS = {"IPHONEOS_DEPLOYMENT_TARGET": "16.0", "MACOSX_DEPLOYMENT_TARGET": "13.0"}


def task_tables() -> dict[str, dict]:
    """Every task mise reads from mise.toml and the TOML files its [task_config] includes."""
    with (ROOT / "mise.toml").open("rb") as config:
        root = tomllib.load(config)
    tasks = dict(root["tasks"])
    for include in root["task_config"]["includes"]:
        path = ROOT / include
        if path.suffix == ".toml":
            with path.open("rb") as included:
                tasks.update(tomllib.load(included))
    return tasks


def test_only_the_swift_build_task_sets_apple_targets() -> None:
    """A C or Ruby compiler must not inherit an iOS target from Swift tasks."""
    with (ROOT / "mise.toml").open("rb") as config:
        assert not set(APPLE_TARGETS) & set(tomllib.load(config).get("env", {}))
    setters = {name for name, task in task_tables().items() if set(APPLE_TARGETS) & set(task.get("env", {}))}
    assert setters == {"swift:build"}
    assert task_tables()["swift:build"]["env"] == APPLE_TARGETS


@pytest.mark.skipif(shutil.which("mise") is None, reason="mise is not installed")
def test_commands_run_through_mise_do_not_inherit_apple_targets() -> None:
    """The repository's mise environment leaves both deployment targets unset."""
    environment = {key: value for key, value in os.environ.items() if key not in APPLE_TARGETS}
    environment.update(MISE_EXEC_AUTO_INSTALL="false", MISE_TASK_RUN_AUTO_INSTALL="false")
    result = subprocess.run(
        ["mise", "exec", "--", "sh", "-c", 'printf "%s|%s" "$IPHONEOS_DEPLOYMENT_TARGET" "$MACOSX_DEPLOYMENT_TARGET"'],
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        check=True,
    )
    assert result.stdout == "|"
