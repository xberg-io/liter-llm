"""Regression coverage for the exact ai-rulez version checks."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import patch

import check_ai_rulez_version as check
import pytest


class TestReportedVersion:
    """The installed version is compared whole, never as a substring."""

    @pytest.mark.parametrize("output", ["ai-rulez version 5.6.0\n", "5.6.0", "v5.6.0", "ai-rulez 5.6.0"])
    def test_accepts_known_forms(self, output: str) -> None:
        assert check.reported_version(output) == "5.6.0"

    @pytest.mark.parametrize("output", ["", "ai-rulez version dev", "ai-rulez version 5.6.0-rc1", "5.6"])
    def test_rejects_unreadable_output(self, output: str) -> None:
        with pytest.raises(SystemExit, match="Cannot read"):
            check.reported_version(output)

    def test_longer_version_is_not_the_pin(self) -> None:
        """15.6.0 contains 5.6.0 but is a different version."""
        assert check.reported_version("ai-rulez version 15.6.0") == "15.6.0"
        assert check.reported_version("ai-rulez version 15.6.0") != "5.6.0"


class TestCheckInstalled:
    """The hook install check runs the real subprocess boundary with a recorded output."""

    def run_with(self, stdout: str) -> None:
        result = subprocess.CompletedProcess([], 0, stdout, "")
        with (
            patch.object(check.shutil, "which", return_value="/bin/ai-rulez"),
            patch.object(check.subprocess, "run", return_value=result),
        ):
            check.check_installed("5.6.0")

    def test_exact_version_passes(self) -> None:
        self.run_with("ai-rulez version 5.6.0\n")

    @pytest.mark.parametrize("stdout", ["ai-rulez version 15.6.0", "ai-rulez version 5.6.01", "ai-rulez version 5.4.1"])
    def test_other_version_fails(self, stdout: str) -> None:
        with pytest.raises(SystemExit):
            self.run_with(stdout)

    def test_missing_binary_fails(self) -> None:
        with patch.object(check.shutil, "which", return_value=None), pytest.raises(SystemExit, match="Install"):
            check.check_installed("5.6.0")


class TestWorkflowPins:
    """Workflow inputs must equal the Taskfile pin."""

    def test_taskfile_pin_is_read(self, tmp_path: Path) -> None:
        taskfile = tmp_path / "Taskfile.yml"
        taskfile.write_text('vars:\n  AI_RULEZ_VERSION: "5.6.0"\n')
        assert check.pinned_version(taskfile) == "5.6.0"

    def test_workflow_inputs_are_collected(self, tmp_path: Path) -> None:
        (tmp_path / "a.yaml").write_text('with:\n  ai-rulez-version: "5.6.0"\n')
        (tmp_path / "b.yml").write_text("with:\n  ai-rulez-version: 5.5.0 # old\n")
        (tmp_path / "c.yml").write_text("name: none\n")
        assert check.workflow_pins(tmp_path) == {"a.yaml": ["5.6.0"], "b.yml": ["5.5.0"]}

    def test_equal_pins_pass(self) -> None:
        check.check_workflows("5.6.0", {"a.yaml": ["5.6.0"]})

    def test_differing_pin_names_the_file(self) -> None:
        with pytest.raises(SystemExit, match="b.yml: 5.5.0"):
            check.check_workflows("5.6.0", {"a.yaml": ["5.6.0"], "b.yml": ["5.5.0"]})

    def test_no_pins_fails(self) -> None:
        with pytest.raises(SystemExit, match="No ai-rulez-version"):
            check.check_workflows("5.6.0", {})

    def test_repository_pins_agree(self) -> None:
        check.check_workflows(check.pinned_version(), check.workflow_pins())
