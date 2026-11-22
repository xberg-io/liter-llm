"""Pinned-version and checksum behaviour of the liter-llm CLI proxy downloader."""

import hashlib
import io
import sys
import tarfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "cli-proxy" / "pypi"))

from liter_llm_cli import __version__, downloader

TRIPLE = "x86_64-unknown-linux-gnu"
SUMS_URL_SUFFIX = f"/v{__version__}/SHA256SUMS-{__version__}.txt"


def _archive_bytes() -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
        payload = b"#!/bin/sh\necho liter-llm\n"
        info = tarfile.TarInfo(f"liter-llm-{__version__}-{TRIPLE}/liter-llm")
        info.size = len(payload)
        info.mode = 0o755
        tar.addfile(info, io.BytesIO(payload))
    return buffer.getvalue()


@pytest.fixture
def linux_env(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Path:
    monkeypatch.setattr(downloader, "_target_triple", lambda: TRIPLE)
    monkeypatch.setattr(downloader, "_binary_name", lambda: "liter-llm")
    monkeypatch.setattr(downloader.Path, "home", lambda: tmp_path)
    monkeypatch.delenv("LITER_LLM_BINARY", raising=False)
    return tmp_path


def test_release_urls_pin_the_package_version() -> None:
    archive, archive_url, checksums_url = downloader.release_urls("2.7.3", "aarch64-apple-darwin")
    assert archive == "liter-llm-2.7.3-aarch64-apple-darwin.tar.gz"
    assert archive_url == (
        "https://github.com/xberg-io/liter-llm/releases/download/v2.7.3/liter-llm-2.7.3-aarch64-apple-darwin.tar.gz"
    )
    assert checksums_url == "https://github.com/xberg-io/liter-llm/releases/download/v2.7.3/SHA256SUMS-2.7.3.txt"


def test_windows_archive_is_a_zip() -> None:
    assert downloader.archive_name("2.7.3", "x86_64-pc-windows-msvc") == "liter-llm-2.7.3-x86_64-pc-windows-msvc.zip"


def test_ensure_binary_downloads_exactly_the_package_version(linux_env: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    archive = _archive_bytes()
    name = f"liter-llm-{__version__}-{TRIPLE}.tar.gz"
    sums = f"{hashlib.sha256(archive).hexdigest()}  {name}\n".encode()
    requested: list[str] = []

    def fake_get(url: str) -> bytes:
        requested.append(url)
        return sums if url.endswith(SUMS_URL_SUFFIX) else archive

    monkeypatch.setattr(downloader, "_http_get", fake_get)
    binary = Path(downloader.ensure_binary())

    assert binary.parent == linux_env / ".cache" / "liter-llm-cli" / __version__
    assert binary.is_file()
    assert all(f"/download/v{__version__}/" in url for url in requested)
    assert not any("latest" in url or "api.github.com" in url for url in requested)


def test_ensure_binary_rejects_a_checksum_mismatch(linux_env: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    archive = _archive_bytes()
    name = f"liter-llm-{__version__}-{TRIPLE}.tar.gz"
    sums = f"{'0' * 64}  {name}\n".encode()
    monkeypatch.setattr(downloader, "_http_get", lambda url: sums if url.endswith(SUMS_URL_SUFFIX) else archive)

    with pytest.raises(RuntimeError, match="checksum mismatch"):
        downloader.ensure_binary()
    assert not (linux_env / ".cache" / "liter-llm-cli" / __version__ / "liter-llm").exists()


@pytest.mark.usefixtures("linux_env")
def test_ensure_binary_refuses_a_missing_checksum_entry(monkeypatch: pytest.MonkeyPatch) -> None:
    archive = _archive_bytes()
    sums = f"{hashlib.sha256(archive).hexdigest()}  unrelated.tar.gz\n".encode()
    monkeypatch.setattr(downloader, "_http_get", lambda url: sums if url.endswith(SUMS_URL_SUFFIX) else archive)

    with pytest.raises(RuntimeError, match="no checksum entry"):
        downloader.ensure_binary()


@pytest.mark.usefixtures("linux_env")
def test_bundled_binary_is_used_without_any_network(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    bundled = tmp_path / "liter-llm"
    bundled.write_text("binary")
    monkeypatch.setattr(downloader, "bundled_binary", lambda: bundled)

    def forbidden(url: str) -> bytes:
        raise AssertionError(f"unexpected download of {url}")

    monkeypatch.setattr(downloader, "_http_get", forbidden)
    assert downloader.ensure_binary() == str(bundled)


def test_binary_override_wins(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("LITER_LLM_BINARY", "/opt/liter-llm")
    assert downloader.ensure_binary() == "/opt/liter-llm"
