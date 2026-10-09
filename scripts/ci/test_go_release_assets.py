"""Verify the release archives satisfy the generated Go setup contract."""

import hashlib
from pathlib import Path

import pytest
from prepare_go_release_assets import prepare

ROOT = Path(__file__).resolve().parents[2]
PUBLISH_WORKFLOW = ROOT / ".github" / "workflows" / "publish.yaml"


def test_alias_preserves_archive_and_writes_installer_checksum(tmp_path: Path) -> None:
    """The installer alias must contain exactly the published native payload."""
    source = tmp_path / "liter-llm-go-v2.1.0-rc.1-macos-arm64.tar.gz"
    payload = b"native archive bytes"
    source.write_bytes(payload)
    prepare(tmp_path, "liter-llm", "2.1.0-rc.1")
    alias = tmp_path / "liter-llm-go-macos-arm64.tar.gz"
    assert source.read_bytes() == alias.read_bytes() == payload
    assert alias.with_name(alias.name + ".sha256").read_text() == (
        f"{hashlib.sha256(payload).hexdigest()}  {alias.name}\n"
    )
    prepare(tmp_path, "liter-llm", "2.1.0-rc.1")
    assert alias.read_bytes() == payload


def test_missing_current_release_archives_fails(tmp_path: Path) -> None:
    """An empty or wrong-version artifact directory cannot pass packaging."""
    (tmp_path / "liter-llm-go-v1.0.0-macos-arm64.tar.gz").write_bytes(b"old")
    with pytest.raises(ValueError, match="No Go release archives"):
        prepare(tmp_path, "liter-llm", "2.0.0")


def test_conflicting_existing_alias_is_not_overwritten(tmp_path: Path) -> None:
    """Recovery must not replace a different already-prepared native payload."""
    (tmp_path / "liter-llm-go-v2.0.0-linux-x86_64.tar.gz").write_bytes(b"new")
    alias = tmp_path / "liter-llm-go-linux-x86_64.tar.gz"
    alias.write_bytes(b"existing")
    with pytest.raises(ValueError, match="differs"):
        prepare(tmp_path, "liter-llm", "2.0.0")
    assert alias.read_bytes() == b"existing"


def test_publish_workflow_builds_both_musl_go_assets() -> None:
    workflow = PUBLISH_WORKFLOW.read_text()
    assert workflow.count("target: x86_64-unknown-linux-musl") == 1
    assert workflow.count("target: aarch64-unknown-linux-musl") == 1
    assert "needs.build-go-musl.result == 'success'" in workflow
    assert '*-linux-x86_64-musl)  lib_dir="linux-x86_64-musl"' in workflow
    assert '*-linux-aarch64-musl) lib_dir="linux-aarch64-musl"' in workflow


def test_go_archives_are_stripped_before_packaging() -> None:
    workflow = PUBLISH_WORKFLOW.read_text()
    assert workflow.count("Strip static archive debug symbols") == 2
    assert workflow.count('archive="target/${TARGET}/release/libliter_llm_ffi.a"') == 2
    assert workflow.index("Strip static archive debug symbols") < workflow.index("Package Go FFI")
