"""Regression coverage for Poly task-file report handling."""

from __future__ import annotations

import contextlib
import io
import json
import subprocess
from unittest.mock import patch

import check_task_lint_coverage as coverage
import pytest


class TestTaskLintCoverage:
    """Keep report parsing fail-closed without losing per-file coverage."""

    def report(self, payload: object, status: int = 0) -> list[dict]:
        """Exercise the actual Poly subprocess boundary with a recorded-shaped response."""
        result = subprocess.CompletedProcess([], status, json.dumps(payload), "")
        with (
            patch.object(coverage.shutil, "which", return_value="poly"),
            patch.object(coverage.subprocess, "run", return_value=result),
        ):
            return coverage.poly_report([coverage.ROOT / "mise.toml"])

    def test_current_envelope_and_legacy_array_preserve_rows(self) -> None:
        """Both supported versions must retain the exact per-file verdicts."""
        rows = [{"path": "mise.toml", "changed": False, "skipped": "parse error"}]
        envelope = {"results": rows, "errors": [], "skipped": [], "summary": {"errored": 0}, "configs": []}
        assert self.report(envelope) == rows
        assert self.report(rows) == rows

    def test_formatter_errors_fail_even_with_complete_results(self) -> None:
        """A surveyed path cannot hide a formatter error in the envelope."""
        with pytest.raises(SystemExit, match="formatter errors"):
            self.report({"results": [{"path": "mise.toml"}], "errors": [{"message": "parse failed"}]}, 1)

    def test_legacy_per_file_error_is_not_a_clean_result(self) -> None:
        """Legacy reports must reject an error recorded alongside a path."""
        with pytest.raises(SystemExit, match="formatter errors"):
            self.report([{"path": "mise.toml", "error": "parse failed"}], 1)

    def test_missing_or_malformed_results_fail(self) -> None:
        """Unknown report shapes cannot become zero-work successes."""
        for payload in ({"errors": []}, {"results": {}}, {"results": [None], "errors": []}):
            with pytest.raises(SystemExit):
                self.report(payload)

    def test_aborted_poly_is_rejected_before_report(self) -> None:
        """A prefix report from an unsuccessful survey is unusable."""
        with pytest.raises(SystemExit, match="did not survey"):
            self.report([{"path": "mise.toml"}], 2)

    def test_main_rejects_missing_paths_and_any_skip(self) -> None:
        """Completeness is enforced, and a skip for any reason fails, after normalization."""
        skip = {"path": str(coverage.ROOT / "mise.toml"), "changed": False}
        for rows in ([], [{**skip, "skipped": "parse error"}], [{**skip, "skipped": "Go/Helm template syntax"}]):
            with (
                patch.object(coverage, "scan_set", return_value=[coverage.ROOT / "mise.toml"]),
                patch.object(coverage, "poly_report", return_value=rows),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                assert coverage.main() == 1

    def test_main_passes_a_complete_report_without_skips(self) -> None:
        """The success path needs every scanned path reported and none skipped."""
        rows = [{"path": str(coverage.ROOT / "mise.toml"), "changed": False}]
        with (
            patch.object(coverage, "scan_set", return_value=[coverage.ROOT / "mise.toml"]),
            patch.object(coverage, "poly_report", return_value=rows),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            assert coverage.main() == 0

    def test_scan_set_covers_every_mise_task_file(self) -> None:
        """The real scan set holds mise.toml, the included TOML task files and the shell file tasks."""
        names = {path.relative_to(coverage.ROOT).as_posix() for path in coverage.scan_set()}
        assert {"mise.toml", "mise-tasks/version.toml", "mise-tasks/test/scripts.sh"} <= names
        assert all(name == "mise.toml" or name.startswith("mise-tasks/") for name in names)
