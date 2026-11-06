"""The lint workflow installs the poly that mise.toml pins, so CI and a local `mise run lint` agree."""

import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

import poly_version
import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]


def mise_pin() -> str:
    with (ROOT / "mise.toml").open("rb") as config:
        return tomllib.load(config)["tools"]["github:Goldziher/poly"]


def test_poly_version_is_the_mise_pin_as_a_release_tag() -> None:
    assert poly_version.poly_version(ROOT / "mise.toml") == f"v{mise_pin()}"


def test_missing_pin_fails(tmp_path: Path) -> None:
    (tmp_path / "mise.toml").write_text('[tools]\nnode = "26.12.1"\n')
    with pytest.raises(ValueError, match=r"pins no github:Goldziher/poly"):
        poly_version.poly_version(tmp_path / "mise.toml")


def test_lint_workflow_passes_the_mise_pin_to_validate() -> None:
    jobs = yaml.safe_load((ROOT / ".github/workflows/ci-lint.yaml").read_text(encoding="utf-8"))["jobs"]
    validate = jobs["validate"]
    assert validate["needs"] == "poly-version"
    assert validate["with"]["poly-version"] == "${{ needs.poly-version.outputs.version }}"
    run = "\n".join(step.get("run", "") for step in jobs["poly-version"]["steps"])
    assert "scripts/ci/poly_version.py" in run


def run_resolve_step(tmp_path: Path, mise_toml: str) -> subprocess.CompletedProcess[str]:
    """Run the workflow's resolve step, as GitHub runs it, against `mise_toml`."""
    jobs = yaml.safe_load((ROOT / ".github/workflows/ci-lint.yaml").read_text(encoding="utf-8"))["jobs"]
    step = next(step for step in jobs["poly-version"]["steps"] if step.get("id") == "pin")
    (tmp_path / "scripts/ci").mkdir(parents=True)
    shutil.copy(ROOT / "scripts/ci/poly_version.py", tmp_path / "scripts/ci/poly_version.py")
    (tmp_path / "mise.toml").write_text(mise_toml)
    output = tmp_path / "github_output"
    output.write_text("")
    return subprocess.run(
        ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", step["run"]],
        cwd=tmp_path,
        env={"PATH": os.environ["PATH"], "GITHUB_OUTPUT": str(output)},
        capture_output=True,
        text=True,
        check=False,
    )


def test_resolve_step_fails_without_a_pin(tmp_path: Path) -> None:
    result = run_resolve_step(tmp_path, '[tools]\nnode = "26.12.1"\n')
    assert result.returncode != 0
    assert "version=" not in (tmp_path / "github_output").read_text()


def test_resolve_step_writes_the_pin(tmp_path: Path) -> None:
    result = run_resolve_step(tmp_path, '[tools]\n"github:Goldziher/poly" = "0.32.5"\n')
    assert result.returncode == 0, result.stderr
    assert (tmp_path / "github_output").read_text() == "version=v0.32.5\n"


@pytest.mark.parametrize(
    "pin",
    ['"latest"', '"0.32"', '{ version = "0.32.5" }', '"v0.32.5-rc.1 junk"'],
)
def test_cli_rejects_a_pin_that_is_not_a_release_version(tmp_path: Path, pin: str) -> None:
    (tmp_path / "mise.toml").write_text(f'[tools]\n"github:Goldziher/poly" = {pin}\n')
    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts/ci/poly_version.py"), str(tmp_path / "mise.toml")],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert result.stdout == ""
    assert "::error::" in result.stderr


def test_cli_exits_1_without_a_pin(tmp_path: Path) -> None:
    (tmp_path / "mise.toml").write_text("[tools]\n")
    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts/ci/poly_version.py"), str(tmp_path / "mise.toml")],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert "pins no github:Goldziher/poly" in result.stderr
