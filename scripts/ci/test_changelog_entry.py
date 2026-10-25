"""Check that release notes start from the version's CHANGELOG entry."""

import subprocess
import sys
from pathlib import Path

import pytest
import yaml
from changelog_entry import changelog_entry

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/publish.yaml"
CHANGELOG = """# Changelog

## [Unreleased]

## [2.4.0] - 2026-10-24

### Changed

- Newer change.

## [2.3.0] - 2026-10-19

### Fixed

- Older fix.

## [2.2.0-rc.1] - 2026-10-01

- Prerelease change.
"""


def test_entry_is_the_text_between_two_headings() -> None:
    assert changelog_entry(CHANGELOG, "2.4.0") == "### Changed\n\n- Newer change."


def test_entry_of_last_version_runs_to_the_end() -> None:
    assert changelog_entry(CHANGELOG, "2.2.0-rc.1") == "- Prerelease change."


def test_version_does_not_match_a_longer_version() -> None:
    with pytest.raises(ValueError, match="no entry"):
        changelog_entry(CHANGELOG, "2.4")


def test_missing_version_is_an_error() -> None:
    with pytest.raises(ValueError, match="no entry for version 9.9.9"):
        changelog_entry(CHANGELOG, "9.9.9")


def test_empty_entry_is_an_error() -> None:
    with pytest.raises(ValueError, match="is empty"):
        changelog_entry(CHANGELOG, "Unreleased")


def test_command_exits_nonzero_for_missing_version(tmp_path: Path) -> None:
    path = tmp_path / "CHANGELOG.md"
    path.write_text(CHANGELOG, encoding="utf-8")
    script = ROOT / "scripts/ci/changelog_entry.py"
    result = subprocess.run([sys.executable, str(script), str(path), "9.9.9"], capture_output=True, text=True)
    assert result.returncode == 1
    assert "::error::" in result.stderr


def test_repository_changelog_has_an_entry_for_its_latest_release() -> None:
    text = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    latest = next(line for line in text.splitlines() if line.startswith("## [") and "Unreleased" not in line)
    assert changelog_entry(text, latest.split("]")[0].removeprefix("## [")).strip()


def test_swift_step_writes_the_changelog_entry_before_the_install_sections() -> None:
    steps = yaml.safe_load(WORKFLOW.read_text())["jobs"]["upload-swift-release"]["steps"]
    step = next(s for s in steps if s.get("name") == "Update release notes with Swift install instructions")
    script = step["run"]
    assert 'scripts/ci/changelog_entry.py CHANGELOG.md "${VERSION}"' in script
    assert script.index("changelog_entry.py") < script.index("## Swift Package Manager")
    assert "cat /tmp/changelog-notes.md /tmp/swift-install.md > /tmp/release-notes.md" in script
    assert "--notes-file /tmp/release-notes.md" in script


def test_zig_notes_are_appended_after_the_swift_rewrite() -> None:
    jobs = yaml.safe_load(WORKFLOW.read_text())["jobs"]
    assert "upload-swift-release" in jobs["publish-zig"]["needs"]
    assert "publish-zig" not in jobs["upload-swift-release"]["needs"]


def test_only_the_swift_step_replaces_the_release_notes() -> None:
    lines = [line.strip() for line in WORKFLOW.read_text().splitlines()]
    commands = [line for line in lines if not line.startswith("#")]
    replacing = [line for line in commands if "gh release edit" in line and "--notes-file" in line]
    assert replacing == ['gh release edit "v${VERSION}" --notes-file /tmp/release-notes.md']
    assert sum(line.startswith("update-release-notes:") for line in commands) == 1
