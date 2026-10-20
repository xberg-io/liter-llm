#!/usr/bin/env python3
"""Fail the build when a Python wheel is not the wheel its matrix leg promises.

A cross-compiled wheel is the one kind of wheel whose name can lie: the platform tag comes from
cibuildwheel's settings while the machine code comes from cargo's target, and nothing ties the
two together. This check reads both from the finished wheel files, so a wrong-architecture
wheel stops the leg before it uploads.

Usage:
    python3 scripts/ci/check_python_wheel.py WHEEL_DIR PLATFORM_TAG_REGEX [MACHO_ARCH]

Every file ``WHEEL_DIR/*.whl`` must have a platform tag that fully matches PLATFORM_TAG_REGEX.
When MACHO_ARCH (``x86_64`` or ``arm64``) is given and not empty, every Mach-O file inside a wheel must contain
that architecture and no other, and the wheel must hold at least one Mach-O file.

A wheel with a macOS platform tag is never checked by its tag alone: the script exits 2 when it finds one and
MACHO_ARCH is missing or empty, so a build step that loses the architecture fails instead of passing. Linux and
Windows legs pass no MACHO_ARCH.
"""

from __future__ import annotations

import re
import struct
import sys
import zipfile
from pathlib import Path

EXPECTED_ARCHS = ("x86_64", "arm64")
CPU_TYPES = {0x01000007: "x86_64", 0x0100000C: "arm64", 0x00000007: "i386", 0x0000000C: "arm"}
THIN_MAGICS = {0xFEEDFACE: "<", 0xFEEDFACF: "<", 0xCEFAEDFE: ">", 0xCFFAEDFE: ">"}
FAT_MAGICS = {0xCAFEBABE: ">", 0xCAFEBABF: ">"}
# 0xCAFEBABE is also the magic of a Java class file; a fat header counts at most a few dozen architectures.
MAX_FAT_ARCHS = 30


def macho_archs(data: bytes) -> list[str] | None:
    """Return the architectures of a Mach-O file, or None when the bytes are not Mach-O."""
    if len(data) < 8:
        return None
    (magic,) = struct.unpack(">I", data[:4])
    if magic in THIN_MAGICS:
        # The magic read big-endian names the byte order of the file: feedface is big-endian.
        order = ">" if magic in (0xFEEDFACE, 0xFEEDFACF) else "<"
        (cpu,) = struct.unpack(order + "I", data[4:8])
        return [CPU_TYPES.get(cpu, f"cpu-{cpu:#x}")]
    if magic in FAT_MAGICS:
        (count,) = struct.unpack(">I", data[4:8])
        if not 0 < count <= MAX_FAT_ARCHS:
            return None
        stride = 32 if magic == 0xCAFEBABF else 20
        if len(data) < 8 + count * stride:
            return None
        return [
            CPU_TYPES.get(cpu, f"cpu-{cpu:#x}")
            for (cpu,) in (struct.unpack(">I", data[8 + i * stride : 12 + i * stride]) for i in range(count))
        ]
    return None


def check_wheel(wheel: Path, tag_pattern: str, expected_arch: str | None) -> list[str]:
    problems: list[str] = []
    platform_tag = wheel.name[: -len(".whl")].split("-")[-1]
    if re.fullmatch(tag_pattern, platform_tag) is None:
        problems.append(f"{wheel.name}: platform tag {platform_tag!r} does not match {tag_pattern!r}")
    if expected_arch is None:
        return problems
    found = 0
    with zipfile.ZipFile(wheel) as archive:
        for member in archive.infolist():
            if member.is_dir():
                continue
            archs = macho_archs(archive.open(member).read(8 + 30 * 32))
            if archs is None:
                continue
            found += 1
            if archs != [expected_arch]:
                problems.append(
                    f"{wheel.name}: {member.filename} has architectures {archs}, expected [{expected_arch!r}]"
                )
    if found == 0:
        problems.append(f"{wheel.name}: no Mach-O file found, so the architecture cannot be proven")
    return problems


def main(argv: list[str]) -> int:
    if len(argv) not in (3, 4):
        print(__doc__, file=sys.stderr)
        return 2
    wheel_dir, tag_pattern = Path(argv[1]), argv[2]
    expected_arch = (argv[3] if len(argv) == 4 else "") or None
    if expected_arch is not None and expected_arch not in EXPECTED_ARCHS:
        print(f"::error::unknown architecture {expected_arch!r}, expected one of {EXPECTED_ARCHS}", file=sys.stderr)
        return 2
    wheels = sorted(wheel_dir.glob("*.whl"))
    if not wheels:
        print(f"no wheel found in {wheel_dir}", file=sys.stderr)
        return 1
    if expected_arch is None:
        macos = [wheel.name for wheel in wheels if wheel.name[: -len(".whl")].split("-")[-1].startswith("macosx_")]
        if macos:
            print(
                f"::error::{macos[0]} is a macOS wheel and no expected architecture was given; "
                "refusing to check it by platform tag alone",
                file=sys.stderr,
            )
            return 2
    problems = [problem for wheel in wheels for problem in check_wheel(wheel, tag_pattern, expected_arch)]
    for problem in problems:
        print(f"::error::{problem}")
    for wheel in wheels:
        print(f"checked {wheel.name}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
