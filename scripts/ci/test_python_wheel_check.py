"""The wheel check must reject a wheel whose machine code is not the architecture its name claims."""

import struct
import subprocess
import sys
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check_python_wheel.py"
X86_64 = 0x01000007
ARM64 = 0x0100000C
LINUX_ELF = b"\x7fELF" + bytes(60)


def thin(cpu: int) -> bytes:
    """A 64-bit little-endian Mach-O header, padded the way a real file is longer than its header."""
    return struct.pack("<IIIIIII", 0xFEEDFACF, cpu, 3, 6, 0, 0, 0) + bytes(64)


def fat(*cpus: int) -> bytes:
    header = struct.pack(">II", 0xCAFEBABE, len(cpus))
    entries = b"".join(struct.pack(">IIIII", cpu, 3, 4096 * (i + 1), 64, 12) for i, cpu in enumerate(cpus))
    return header + entries + bytes(64)


def fat64(*cpus: int) -> bytes:
    """A 64-bit fat header: 32-byte entries with 64-bit offset and size fields, after the magic 0xCAFEBABF."""
    header = struct.pack(">II", 0xCAFEBABF, len(cpus))
    entries = b"".join(struct.pack(">IIQQII", cpu, 3, 4096 * (i + 1), 64, 12, 0) for i, cpu in enumerate(cpus))
    return header + entries + bytes(64)


def java_class(major: int) -> bytes:
    """A Java class file: the fat Mach-O magic, then a minor and a major version that read as one 32-bit count."""
    return struct.pack(">IHH", 0xCAFEBABE, 0, major) + bytes(1300)


def make_wheel(directory: Path, platform: str, members: dict[str, bytes], version: str = "2.2.5") -> Path:
    wheel = directory / f"liter_llm-{version}-cp310-abi3-{platform}.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        for name, data in members.items():
            archive.writestr(name, data)
    return wheel


def run(directory: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(directory), *args], capture_output=True, text=True, check=False
    )


def test_x86_64_wheel_with_x86_64_code_passes(tmp_path: Path) -> None:
    make_wheel(
        tmp_path,
        "macosx_11_0_x86_64",
        {"liter_llm/_internal_bindings.abi3.so": thin(X86_64), "liter_llm/__init__.py": b""},
    )
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 0, result.stdout + result.stderr


def test_x86_64_tag_over_arm64_code_fails(tmp_path: Path) -> None:
    """The failure this check exists for: the right name over the wrong machine code."""
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"liter_llm/_internal_bindings.abi3.so": thin(ARM64)})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "has architectures ['arm64']" in result.stdout


def test_one_wrong_file_among_right_files_fails(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": thin(X86_64), "b.dylib": thin(ARM64)})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "b.dylib" in result.stdout
    assert "a.so has" not in result.stdout


def test_universal_file_fails_because_it_holds_a_second_architecture(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": fat(X86_64, ARM64)})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "['x86_64', 'arm64']" in result.stdout


def test_single_architecture_fat_file_passes(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": fat(X86_64)})
    assert run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64").returncode == 0


def test_universal_fat64_file_fails_with_both_architectures_named(tmp_path: Path) -> None:
    """A 64-bit fat header has 32-byte entries; read at the 32-bit stride, the second entry is not an architecture."""
    universal = fat64(X86_64, ARM64)
    assert universal[:4] == bytes.fromhex("cafebabf")
    assert struct.unpack(">I", universal[8 + 32 : 12 + 32]) == (ARM64,)
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": universal})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "a.so has architectures ['x86_64', 'arm64']" in result.stdout
    single = tmp_path / "single"
    single.mkdir()
    make_wheel(single, "macosx_11_0_arm64", {"a.so": fat64(ARM64)})
    assert run(single, r"macosx_\d+_\d+_arm64", "arm64").returncode == 0


def test_java_class_file_is_not_read_as_a_fat_mach_o(tmp_path: Path) -> None:
    """A class file starts with the fat magic, and its version words read as a count of 48 architectures."""
    member = java_class(48)
    assert struct.unpack(">II", member[:8]) == (0xCAFEBABE, 48)
    assert len(member) >= 8 + 48 * 20
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": thin(X86_64), "lib/Tool.class": member})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 0, result.stdout + result.stderr
    control = tmp_path / "control"
    control.mkdir()
    make_wheel(control, "macosx_11_0_x86_64", {"a.so": thin(X86_64), "lib/Tool.class": fat(ARM64)})
    result = run(control, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "lib/Tool.class has architectures ['arm64']" in result.stdout


def test_large_file_with_the_wrong_architecture_fails(tmp_path: Path) -> None:
    """The check reads the header of every file; a cut-off on the size would let a big binary through unread."""
    make_wheel(
        tmp_path,
        "macosx_11_0_x86_64",
        {"a.so": thin(X86_64), "liter_llm/big.dylib": thin(ARM64) + bytes(2_000_000)},
    )
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "liter_llm/big.dylib has architectures ['arm64']" in result.stdout


def test_wrong_platform_tag_fails_even_with_right_code(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_arm64", {"a.so": thin(X86_64)})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "platform tag 'macosx_11_0_arm64'" in result.stdout


def test_wheel_with_no_mach_o_file_fails(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"liter_llm/__init__.py": b"", "a.so": LINUX_ELF})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "no Mach-O file" in result.stdout


def test_empty_directory_fails(tmp_path: Path) -> None:
    assert run(tmp_path, r".*", "x86_64").returncode == 1


def test_tag_only_check_ignores_machine_code(tmp_path: Path) -> None:
    make_wheel(tmp_path, "manylinux_2_17_x86_64", {"a.so": LINUX_ELF})
    assert run(tmp_path, r"manylinux.*_x86_64").returncode == 0
    assert run(tmp_path, r"manylinux.*_aarch64").returncode == 1


def test_empty_architecture_argument_reads_tags_only(tmp_path: Path) -> None:
    make_wheel(tmp_path, "win_amd64", {"a.pyd": LINUX_ELF})
    assert run(tmp_path, "win_amd64", "").returncode == 0


def test_macos_wheel_is_refused_without_an_architecture(tmp_path: Path) -> None:
    """A step that loses the architecture must fail, not fall back to reading the tag."""
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": thin(ARM64)})
    for args in ((r"macosx_\d+_\d+_x86_64",), (r"macosx_\d+_\d+_x86_64", "")):
        result = run(tmp_path, *args)
        assert result.returncode == 2, result.stdout + result.stderr
        assert "no expected architecture" in result.stderr


def test_unknown_architecture_is_refused(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": thin(X86_64)})
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86-64")
    assert result.returncode == 2
    assert "unknown architecture" in result.stderr


def test_tag_must_match_the_whole_pattern_not_a_prefix(tmp_path: Path) -> None:
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": thin(X86_64)})
    assert run(tmp_path, "macosx_11_0_x86_64", "x86_64").returncode == 0
    result = run(tmp_path, "macosx_11_0_x86", "x86_64")
    assert result.returncode == 1
    assert "does not match 'macosx_11_0_x86'" in result.stdout


def test_binary_in_the_data_directory_is_read(tmp_path: Path) -> None:
    make_wheel(
        tmp_path,
        "macosx_11_0_x86_64",
        {"liter_llm/_internal_bindings.abi3.so": thin(X86_64), "liter_llm-2.2.5.data/scripts/tool": thin(ARM64)},
    )
    result = run(tmp_path, r"macosx_\d+_\d+_x86_64", "x86_64")
    assert result.returncode == 1
    assert "liter_llm-2.2.5.data/scripts/tool has architectures ['arm64']" in result.stdout


def test_every_macos_deployment_target_needs_an_architecture(tmp_path: Path) -> None:
    for platform in ("macosx_10_12_x86_64", "macosx_14_0_arm64"):
        directory = tmp_path / platform
        directory.mkdir()
        make_wheel(directory, platform, {"a.so": thin(ARM64)})
        result = run(directory, "macosx_.*")
        assert result.returncode == 2, (platform, result.stdout + result.stderr)
        assert "no expected architecture" in result.stderr


def test_one_macos_wheel_among_linux_wheels_needs_an_architecture(tmp_path: Path) -> None:
    make_wheel(tmp_path, "manylinux_2_28_x86_64", {"a.so": LINUX_ELF})
    assert run(tmp_path, ".*").returncode == 0
    make_wheel(tmp_path, "macosx_11_0_x86_64", {"a.so": thin(ARM64)})
    result = run(tmp_path, ".*")
    assert result.returncode == 2, result.stdout + result.stderr
    assert "no expected architecture" in result.stderr


def test_every_wheel_in_the_directory_is_checked(tmp_path: Path) -> None:
    """A wrong wheel fails the check wherever it sorts in the directory, not only when it is read first."""
    for wrong in ("2.2.5", "2.2.6"):
        directory = tmp_path / wrong
        directory.mkdir()
        wheels = [
            make_wheel(directory, "macosx_11_0_x86_64", {"a.so": thin(ARM64 if version == wrong else X86_64)}, version)
            for version in ("2.2.5", "2.2.6")
        ]
        result = run(directory, r"macosx_\d+_\d+_x86_64", "x86_64")
        assert result.returncode == 1, (wrong, result.stdout + result.stderr)
        assert f"liter_llm-{wrong}-cp310-abi3-macosx_11_0_x86_64.whl: a.so has architectures ['arm64']" in result.stdout
        for wheel in wheels:
            assert f"checked {wheel.name}" in result.stdout, (wrong, wheel.name)
