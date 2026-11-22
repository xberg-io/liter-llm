"""Custom hatchling build hook that bundles the native liter-llm binary into a wheel.

When building a platform-specific wheel in CI, the target triple is supplied via
the ``LITER_LLM_CLI_TARGET`` env var. The matching ``liter-llm-cli-<target>.tar.gz`` /
``.zip`` (the release archive, staged under that name by the publish workflow) is
located in the repo root or ``dist/``, the binary is extracted into
``liter_llm_cli/bin/<target>/``, force-included in the wheel, and the wheel is
tagged for that platform so PyPI serves the right artifact.

If no target is set (the sdist build, or a local build), the hook is a no-op and the
package falls back to the pinned, checksum-verified runtime downloader in
``liter_llm_cli/downloader.py``.
"""

from __future__ import annotations

import os
import shutil
import tarfile
import zipfile
from pathlib import Path

from hatchling.builders.hooks.plugin.interface import BuildHookInterface

# ~keep The linux binaries are built on ubuntu-latest and import GLIBC_2.39 symbols, so the honest
# manylinux floor is 2_39. A 2_28 tag would let pip install a wheel that fails to start on older glibc.
_TAG_MAP = {
    "x86_64-pc-windows-msvc": "win_amd64",
    "x86_64-unknown-linux-gnu": "manylinux_2_39_x86_64",
    "aarch64-unknown-linux-gnu": "manylinux_2_39_aarch64",
    "aarch64-apple-darwin": "macosx_15_0_arm64",
}


def _safe_destination(root: Path, member_name: str) -> Path:
    """Resolve an archive member path and require it to stay under root."""
    root = root.resolve()
    target = (root / member_name.replace("\\", "/")).resolve(strict=False)
    if target != root and not target.is_relative_to(root):
        raise RuntimeError(f"archive member escapes extraction directory: {member_name}")
    return target


def _extract_zip_bounded(archive: Path, extract_dir: Path) -> None:
    """Extract zip entries after bounding each destination path."""
    with zipfile.ZipFile(archive) as zf:
        for member in zf.infolist():
            target = _safe_destination(extract_dir, member.filename)
            if member.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue

            target.parent.mkdir(parents=True, exist_ok=True)
            with zf.open(member) as source, target.open("wb") as destination:
                shutil.copyfileobj(source, destination)


def _extract_tar_bounded(archive: Path, extract_dir: Path) -> None:
    """Extract regular tar entries after bounding each destination path."""
    with tarfile.open(archive, "r:gz") as tf:
        for member in tf.getmembers():
            target = _safe_destination(extract_dir, member.name)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            if not member.isfile():
                raise RuntimeError(f"unsupported archive member type: {member.name}")

            target.parent.mkdir(parents=True, exist_ok=True)
            source = tf.extractfile(member)
            if source is None:
                raise RuntimeError(f"could not read archive member: {member.name}")
            with source, target.open("wb") as destination:
                shutil.copyfileobj(source, destination)


class CustomBuildHook(BuildHookInterface):
    """Inject the matching native binary into a platform-tagged wheel."""

    PLUGIN_NAME = "custom"

    def initialize(self, version: str, build_data: dict) -> None:  # noqa: ARG002
        """Bundle a staged native binary when building a targeted wheel."""
        target = os.environ.get("LITER_LLM_CLI_TARGET", "").strip()
        if not target:
            return

        wheel_tag = _TAG_MAP.get(target)
        if wheel_tag is None:
            raise RuntimeError(f"no wheel platform tag mapped for target {target}")

        archive = self._find_archive(target)
        if archive is None:
            raise RuntimeError(
                f"LITER_LLM_CLI_TARGET={target} but no liter-llm-cli-{target}.(tar.gz|zip) "
                f"found in repo root or dist/; refusing to build an empty platform wheel."
            )

        binary = self._extract_binary(archive, target)
        force_include = build_data.setdefault("force_include", {})
        force_include[str(binary)] = f"liter_llm_cli/bin/{target}/{binary.name}"

        build_data["pure_python"] = False
        build_data["infer_tag"] = False
        build_data["tag"] = f"py3-none-{wheel_tag}"

    def _find_archive(self, target: str) -> Path | None:
        root = Path(self.root)
        repo_root = root.parent.parent
        for base in (repo_root, repo_root / "dist", root, root / "dist"):
            for ext in ("tar.gz", "zip"):
                candidate = base / f"liter-llm-cli-{target}.{ext}"
                if candidate.is_file():
                    return candidate
        return None

    def _extract_binary(self, archive: Path, target: str) -> Path:
        binary_name = "liter-llm.exe" if target.endswith("windows-msvc") else "liter-llm"
        extract_dir = Path(self.root) / ".build-extract" / target
        if extract_dir.exists():
            shutil.rmtree(extract_dir, ignore_errors=True)
        extract_dir.mkdir(parents=True, exist_ok=True)

        if str(archive).lower().endswith(".zip"):
            _extract_zip_bounded(archive, extract_dir)
        else:
            _extract_tar_bounded(archive, extract_dir)

        for candidate in extract_dir.rglob(binary_name):
            if candidate.is_file():
                candidate.chmod(0o755)
                return candidate
        raise RuntimeError(f"binary {binary_name} not found inside {archive.name}")
