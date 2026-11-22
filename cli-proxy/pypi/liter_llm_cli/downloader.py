"""Resolve, download, verify, and run the native liter-llm binary.

The package is published in lockstep with the product, so its own version names
the one release it may install: a binary bundled in the wheel is used first,
otherwise exactly ``liter-llm-<version>-<triple>`` is downloaded from the
``v<version>`` release and verified against ``SHA256SUMS-<version>.txt``. It
never resolves "latest". All diagnostics go to stderr.
"""

from __future__ import annotations

import hashlib
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath
from urllib.error import URLError
from urllib.parse import quote, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

from . import __version__

REPO = "xberg-io/liter-llm"
BIN_NAME = "liter-llm"
PKG_NAME = "liter-llm-cli"
_USER_AGENT = "liter-llm-cli-python-proxy"


def _target_triple() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "windows":
        if machine in {"amd64", "x86_64"}:
            return "x86_64-pc-windows-msvc"
        raise RuntimeError(f"unsupported Windows arch: {machine}")
    if system == "linux":
        if machine in {"amd64", "x86_64"}:
            return "x86_64-unknown-linux-gnu"
        if machine in {"aarch64", "arm64"}:
            return "aarch64-unknown-linux-gnu"
        raise RuntimeError(f"unsupported Linux arch: {machine}")
    if system == "darwin":
        if machine in {"aarch64", "arm64"}:
            return "aarch64-apple-darwin"
        if machine in {"amd64", "x86_64"}:
            return "x86_64-apple-darwin"
        raise RuntimeError(f"unsupported macOS arch: {machine}")
    raise RuntimeError(f"unsupported platform: {system} {machine}")


def _binary_name() -> str:
    return f"{BIN_NAME}.exe" if platform.system().lower() == "windows" else BIN_NAME


class _HttpsOnlyRedirectHandler(HTTPRedirectHandler):
    """Reject any redirect whose target is not https (downgrade/SSRF guard)."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[no-untyped-def]  # noqa: PLR0917 — overrides fixed HTTPRedirectHandler signature
        if urlsplit(newurl).scheme.lower() != "https":
            raise URLError(f"refusing non-https redirect to: {newurl}")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


_opener = build_opener(_HttpsOnlyRedirectHandler())


def _http_get(url: str) -> bytes:
    if urlsplit(url).scheme.lower() != "https":
        raise RuntimeError(f"refusing non-https URL: {url}")
    request = Request(url, headers={"User-Agent": _USER_AGENT})
    try:
        with _opener.open(request, timeout=60) as response:
            if response.status != 200:
                raise RuntimeError(f"HTTP {response.status} for {url}")
            return response.read()  # type: ignore[no-any-return]
    except URLError as exc:
        raise RuntimeError(f"failed to fetch {url}: {exc}") from exc


def archive_name(version: str, triple: str) -> str:
    """Return the release archive name for ``version`` and ``triple``."""
    extension = "zip" if triple.endswith("-windows-msvc") else "tar.gz"
    return f"{BIN_NAME}-{version}-{triple}.{extension}"


def release_urls(version: str, triple: str) -> tuple[str, str, str]:
    """Return (archive name, archive URL, checksums URL) pinned to ``v<version>``."""
    base = f"https://github.com/{REPO}/releases/download/v{quote(version, safe='')}"
    archive = archive_name(version, triple)
    return archive, f"{base}/{archive}", f"{base}/SHA256SUMS-{version}.txt"


def _expected_digest(text: str, asset_name: str) -> str | None:
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        parts = stripped.split()
        if len(parts) < 2:
            continue
        name = parts[-1].lstrip("*")
        if name == asset_name:
            return parts[0].lower()
    return None


def verify_checksum(data: bytes, asset_name: str, sums_text: str) -> None:
    """Raise unless ``data`` matches the digest listed for ``asset_name``."""
    expected = _expected_digest(sums_text, asset_name)
    if not expected:
        raise RuntimeError(f"no checksum entry for {asset_name} in SHA256SUMS — refusing to install unverified binary")
    digest = hashlib.sha256(data).hexdigest().lower()
    if digest != expected:
        raise RuntimeError(f"checksum mismatch for {asset_name} (expected {expected}, got {digest})")


def _is_within(base: Path, target: Path) -> bool:
    """True if `target`'s resolved path stays inside `base`."""
    base_resolved = base.resolve()
    try:
        target.resolve().relative_to(base_resolved)
    except ValueError:
        return False
    return True


def _reject_unsafe_member(name: str) -> None:
    """Reject absolute paths and any `..` component (zip-slip / tar-slip)."""
    normalized = name.replace("\\", "/")
    pure = PurePosixPath(normalized)
    if pure.is_absolute() or normalized.startswith("/"):
        raise RuntimeError(f"refusing absolute path in archive: {name}")
    if len(normalized) >= 2 and normalized[1] == ":":
        raise RuntimeError(f"refusing absolute path in archive: {name}")
    if any(part == ".." for part in pure.parts):
        raise RuntimeError(f"refusing parent-directory escape in archive: {name}")


def _safe_extract(archive_path: Path, asset_name: str, dest: Path) -> None:
    """Extract a tar/zip, validating every member stays within `dest`.

    Does not rely on extractall() defaults or the 3.12-only ``filter='data'``:
    each member name is rejected up front if it is absolute or contains ``..``,
    and the resolved destination path is re-checked against ``dest``.
    """
    dest = dest.resolve()
    dest.mkdir(parents=True, exist_ok=True)

    if asset_name.lower().endswith(".zip"):
        with zipfile.ZipFile(archive_path) as zf:
            for member in zf.namelist():
                _reject_unsafe_member(member)
                if not _is_within(dest, dest / member):
                    raise RuntimeError(f"refusing archive entry escaping dest: {member}")
            for member in zf.namelist():
                zf.extract(member, dest)
    else:
        with tarfile.open(archive_path, "r:gz") as tf:
            for tar_member in tf.getmembers():
                _reject_unsafe_member(tar_member.name)
                if not _is_within(dest, dest / tar_member.name):
                    raise RuntimeError(f"refusing archive entry escaping dest: {tar_member.name}")
                if tar_member.islnk() or tar_member.issym():
                    link_target = dest / tar_member.name
                    if not _is_within(dest, link_target.parent / tar_member.linkname):
                        raise RuntimeError(f"refusing link escaping dest: {tar_member.name}")
            tf.extractall(dest)


def _find_binary(root: Path, name: str) -> Path | None:
    for candidate in root.rglob(name):
        if candidate.is_file():
            return candidate
    return None


def _cache_dir(version: str) -> Path:
    cache = Path.home() / ".cache" / PKG_NAME / version
    cache.mkdir(parents=True, exist_ok=True, mode=0o700)
    return cache


def bundled_binary() -> Path | None:
    """Return the binary shipped inside a platform wheel, if there is one."""
    candidate = Path(__file__).resolve().parent / "bin" / _target_triple() / _binary_name()
    return candidate if candidate.is_file() else None


def ensure_binary() -> str:
    """Ensure the native binary for this package's version exists, downloading if necessary."""
    override = os.getenv("LITER_LLM_BINARY")
    if override:
        return override

    bundled = bundled_binary()
    if bundled is not None:
        return str(bundled)

    version = __version__
    triple = _target_triple()
    cache = _cache_dir(version)
    binary_path = cache / _binary_name()
    if binary_path.exists() and os.access(binary_path, os.X_OK):
        return str(binary_path)

    archive, archive_url, checksums_url = release_urls(version, triple)
    print(f"Downloading {BIN_NAME} {version} asset {archive}...", file=sys.stderr)
    data = _http_get(archive_url)
    verify_checksum(data, archive, _http_get(checksums_url).decode("utf-8"))
    print(f"Checksum verified for {archive}.", file=sys.stderr)
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        archive_path = tmp / archive
        archive_path.write_bytes(data)
        extract_dir = tmp / "extract"
        extract_dir.mkdir()
        _safe_extract(archive_path, archive, extract_dir)
        found = _find_binary(extract_dir, _binary_name())
        if not found:
            raise RuntimeError(f"binary {_binary_name()} not found after extracting {archive}")
        shutil.move(str(found), str(binary_path))

    if platform.system().lower() != "windows":
        binary_path.chmod(0o755)
    print(f"{BIN_NAME} installed.", file=sys.stderr)
    return str(binary_path)


def run(args: list[str]) -> int:
    """Run the native binary with the given args, returning its exit code."""
    binary_path = ensure_binary()
    completed = subprocess.run([binary_path, *args], check=False)
    return completed.returncode
