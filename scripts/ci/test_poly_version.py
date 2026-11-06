"""The lint workflow installs the poly that mise.toml pins, so CI and a local `mise run lint` agree."""

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
