"""Regression coverage for the ai-rulez latest-input check."""

from __future__ import annotations

from pathlib import Path

import check_ai_rulez_version as check
import pytest


class TestWorkflowPins:
    """Every workflow input must be latest."""

    def test_workflow_inputs_are_collected(self, tmp_path: Path) -> None:
        (tmp_path / "a.yaml").write_text('with:\n  ai-rulez-version: "latest"\n')
        (tmp_path / "b.yml").write_text("with:\n  ai-rulez-version: 5.7.0 # old\n")
        (tmp_path / "c.yml").write_text("name: none\n")
        assert check.workflow_pins(tmp_path) == {"a.yaml": ["latest"], "b.yml": ["5.7.0"]}

    def test_latest_inputs_pass(self) -> None:
        check.check_workflows({"a.yaml": ["latest", "latest"]})

    def test_exact_pin_names_the_file(self) -> None:
        with pytest.raises(SystemExit, match=r"b\.yml: 5\.7\.0"):
            check.check_workflows({"a.yaml": ["latest"], "b.yml": ["5.7.0"]})

    def test_no_inputs_fails(self) -> None:
        with pytest.raises(SystemExit, match="No ai-rulez-version"):
            check.check_workflows({})

    def test_unknown_mode_fails(self) -> None:
        with pytest.raises(SystemExit, match="usage"):
            check.main(["check_ai_rulez_version.py", "installed"])

    def test_repository_inputs_are_latest(self) -> None:
        pins = check.workflow_pins()
        assert sum(len(values) for values in pins.values()) == 8
        check.check_workflows(pins)
