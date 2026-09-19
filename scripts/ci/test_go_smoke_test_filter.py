"""Keep the Go smoke task's `-run` filter aligned with the generated smoke suite."""

import re
from pathlib import Path

import pytest
from go_smoke_test_filter import DEFAULT_SOURCE, smoke_test_filter


def test_filter_selects_every_test_in_the_generated_smoke_suite() -> None:
    """Regeneration must not leave the filter matching a subset (or none) of smoke_test.go."""
    declared = re.findall(r"^func (Test\w*)\(", DEFAULT_SOURCE.read_text(), flags=re.MULTILINE)
    pattern = re.compile(smoke_test_filter(DEFAULT_SOURCE))
    assert len(declared) > 0
    assert [name for name in declared if pattern.fullmatch(name)] == declared


def test_filter_is_anchored_to_exact_names(tmp_path: Path) -> None:
    """A prefix such as `Test_Smoke` must not leak sibling files' tests into the smoke run."""
    source = tmp_path / "smoke_test.go"
    source.write_text(
        "package e2e_test\n\nfunc Test_Smoke(t *testing.T) {}\nfunc Test_Other(tb *testing.T) {}\n"
        "func helper(t *testing.T) {}\nfunc TestMain(m *testing.M) {}\n"
    )
    assert smoke_test_filter(source) == "^(Test_Smoke|Test_Other)$"
    pattern = re.compile(smoke_test_filter(source))
    assert pattern.fullmatch("Test_SmokeBatchCompleted") is None


def test_file_without_tests_is_rejected(tmp_path: Path) -> None:
    """A filter that would execute zero tests must fail instead of passing vacuously."""
    source = tmp_path / "smoke_test.go"
    source.write_text("package e2e_test\n\nfunc TestMain(m *testing.M) {}\n")
    with pytest.raises(SystemExit, match="zero tests"):
        smoke_test_filter(source)
