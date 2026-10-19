"""Keep the manual `php:test` smoke target runnable and honest about failures."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest
import tomllib

ROOT = Path(__file__).resolve().parents[2]
SMOKE_SH = ROOT / "scripts/php/smoke.sh"
SMOKE_PHP = ROOT / "scripts/php/smoke.php"
WORKSPACE_VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]


def fake_php(tmp_path: Path, exit_code: int) -> Path:
    """A stand-in interpreter that records its arguments and exits with `exit_code`."""
    script = tmp_path / "php"
    script.write_text(f'#!/usr/bin/env bash\nprintf "%s\\n" "$@" > "{tmp_path}/args"\nexit {exit_code}\n')
    script.chmod(0o755)
    return script


def run_smoke(**env: str) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment.update(env)
    return subprocess.run(
        ["bash", str(SMOKE_SH)],
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )


def test_smoke_files_exist() -> None:
    assert SMOKE_SH.is_file()
    assert SMOKE_PHP.is_file()


def test_missing_extension_fails_with_build_hint(tmp_path: Path) -> None:
    result = run_smoke(LITER_LLM_PHP_EXT=str(tmp_path / "missing.so"), PHP=str(fake_php(tmp_path, 0)))
    assert result.returncode != 0
    assert "task php:build" in result.stderr
    assert not (tmp_path / "args").exists(), "PHP must not run without an extension"


@pytest.mark.parametrize("exit_code", [0, 1, 3])
def test_loads_extension_passes_version_and_propagates_status(tmp_path: Path, exit_code: int) -> None:
    extension = tmp_path / "libliter_llm_php.so"
    extension.write_bytes(b"")
    result = run_smoke(LITER_LLM_PHP_EXT=str(extension), PHP=str(fake_php(tmp_path, exit_code)))
    assert result.returncode == exit_code
    args = (tmp_path / "args").read_text().splitlines()
    assert args == ["-n", "-d", f"extension={extension}", str(SMOKE_PHP), WORKSPACE_VERSION]


@pytest.mark.skipif(shutil.which("php") is None, reason="php is not installed")
def test_smoke_script_fails_when_extension_is_not_loaded() -> None:
    result = subprocess.run(
        ["php", "-n", str(SMOKE_PHP), WORKSPACE_VERSION],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert "not loaded" in result.stderr


@pytest.mark.skipif(shutil.which("task") is None, reason="task is not installed")
def test_task_dispatch_runs_the_smoke_script(tmp_path: Path) -> None:
    taskfile = tmp_path / "Taskfile.yml"
    taskfile.write_text(f'version: "3"\nincludes:\n  php: "{ROOT / ".task/languages/php.yml"}"\n')
    environment = os.environ.copy()
    environment.update(LITER_LLM_PHP_EXT=str(tmp_path / "missing.so"), PHP=str(fake_php(tmp_path, 0)))
    result = subprocess.run(
        ["task", "--taskfile", str(taskfile), "php:test"],
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode != 0
    assert "liter_llm extension not found" in result.stderr
