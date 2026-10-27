"""Regression coverage for the ai-rulez minimum-version and latest-input checks."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import patch

import check_ai_rulez_version as check
import pytest


class TestReportedVersion:
    """The installed version is parsed whole and compared numerically."""

    @pytest.mark.parametrize("output", ["ai-rulez version 5.7.0\n", "5.7.0", "v5.7.0", "ai-rulez 5.7.0"])
    def test_accepts_known_forms(self, output: str) -> None:
        assert check.reported_version(output) == (5, 7, 0)

    @pytest.mark.parametrize(
        "output", ["", "dev", "ai-rulez version dev", "ai-rulez version 5.7.0-rc1", "5.7", "v5.7.0.1"]
    )
    def test_rejects_unreadable_output(self, output: str) -> None:
        with pytest.raises(SystemExit, match="Cannot read"):
            check.reported_version(output)

    def test_compares_numerically_not_lexically(self) -> None:
        """5.10.0 sorts before 5.7.0 as text but is newer."""
        assert check.reported_version("5.10.0") > check.reported_version("5.7.0")


class TestMinimumVersion:
    """The floor comes from the Taskfile and must be a plain release."""

    def test_reads_the_taskfile_minimum(self, tmp_path: Path) -> None:
        taskfile = tmp_path / "Taskfile.yml"
        taskfile.write_text('vars:\n  AI_RULEZ_MIN_VERSION: "5.7.0"\n')
        assert check.minimum_version(taskfile) == (5, 7, 0)

    @pytest.mark.parametrize("text", ["vars:\n", 'vars:\n  AI_RULEZ_MIN_VERSION: "latest"\n'])
    def test_missing_or_non_numeric_minimum_fails(self, tmp_path: Path, text: str) -> None:
        taskfile = tmp_path / "Taskfile.yml"
        taskfile.write_text(text)
        with pytest.raises(SystemExit, match="AI_RULEZ_MIN_VERSION"):
            check.minimum_version(taskfile)

    def test_repository_minimum_is_readable(self) -> None:
        assert check.minimum_version() >= (5, 7, 0)


class TestCheckInstalled:
    """The hook install check runs the real subprocess boundary with a recorded output."""

    def run_with(self, stdout: str) -> None:
        result = subprocess.CompletedProcess([], 0, stdout, "")
        with (
            patch.object(check.shutil, "which", return_value="/bin/ai-rulez"),
            patch.object(check.subprocess, "run", return_value=result),
        ):
            check.check_installed((5, 7, 0))

    @pytest.mark.parametrize("stdout", ["ai-rulez version 5.7.0\n", "ai-rulez version 5.10.0", "6.0.0"])
    def test_minimum_or_newer_passes(self, stdout: str) -> None:
        self.run_with(stdout)

    @pytest.mark.parametrize("stdout", ["ai-rulez version 5.6.10", "4.99.0", "ai-rulez version dev"])
    def test_older_or_unreadable_fails(self, stdout: str) -> None:
        with pytest.raises(SystemExit):
            self.run_with(stdout)

    def test_missing_binary_fails(self) -> None:
        with patch.object(check.shutil, "which", return_value=None), pytest.raises(SystemExit, match="Install"):
            check.check_installed((5, 7, 0))


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

    def test_repository_inputs_are_latest(self) -> None:
        pins = check.workflow_pins()
        assert sum(len(values) for values in pins.values()) == 8
        check.check_workflows(pins)
