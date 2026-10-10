"""Reject an unexpectedly large native library before release packaging."""

from __future__ import annotations

import argparse
from pathlib import Path


def validate_library_size(path: Path, *, max_bytes: int) -> int:
    """Return the file size when it is within budget."""
    if max_bytes <= 0:
        raise ValueError("max_bytes must be positive")
    if not path.is_file():
        raise FileNotFoundError(f"native library does not exist: {path}")

    size = path.stat().st_size
    if size > max_bytes:
        raise ValueError(f"native library {path} is {size} bytes and exceeds the {max_bytes}-byte budget")
    return size


def main() -> None:
    """Validate one native library supplied by the release workflow."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library", type=Path)
    parser.add_argument("--max-mib", type=int, required=True)
    args = parser.parse_args()

    max_bytes = args.max_mib * 1024 * 1024
    size = validate_library_size(args.library, max_bytes=max_bytes)
    print(f"{args.library}: {size / (1024 * 1024):.1f} MiB (limit {args.max_mib} MiB)")


if __name__ == "__main__":
    main()
