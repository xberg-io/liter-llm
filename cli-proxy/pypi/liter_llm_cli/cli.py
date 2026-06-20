"""CLI entry point for the liter-llm proxy."""

from __future__ import annotations

import sys

from .downloader import run


def main() -> None:
    """Resolve the native binary and exec it with forwarded argv."""
    sys.exit(run(sys.argv[1:]))


if __name__ == "__main__":
    main()
