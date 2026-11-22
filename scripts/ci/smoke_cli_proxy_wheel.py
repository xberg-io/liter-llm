#!/usr/bin/env python3
"""Install a built liter-llm-cli wheel into a clean venv and run the binary it ships.

Usage: smoke_cli_proxy_wheel.py <wheel> <expected-version>

Fails unless the wheel carries its own binary (so a wheel that would fall back to a runtime
download cannot pass on the strength of the network) and that binary reports the release version.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 3:
        sys.exit("usage: smoke_cli_proxy_wheel.py <wheel> <expected-version>")
    wheel, expected = Path(sys.argv[1]).resolve(), sys.argv[2]
    if not wheel.is_file():
        sys.exit(f"wheel not found: {wheel}")

    with tempfile.TemporaryDirectory() as tmp:
        venv = Path(tmp) / "venv"
        subprocess.run([sys.executable, "-m", "venv", str(venv)], check=True)
        bin_dir = venv / ("Scripts" if os.name == "nt" else "bin")
        python = bin_dir / ("python.exe" if os.name == "nt" else "python")
        subprocess.run([str(python), "-m", "pip", "install", "--quiet", "--no-deps", str(wheel)], check=True)

        env = {k: v for k, v in os.environ.items() if k != "LITER_LLM_BINARY"}
        probe = "from liter_llm_cli import downloader; b = downloader.bundled_binary(); print(b or ''); raise SystemExit(0 if b else 1)"
        bundled = subprocess.run([str(python), "-c", probe], env=env, capture_output=True, text=True, check=False)
        if bundled.returncode != 0:
            print(f"::error::{wheel.name} ships no bundled binary for this platform", file=sys.stderr)
            return 1

        launcher = bin_dir / ("liter-llm.exe" if os.name == "nt" else "liter-llm")
        result = subprocess.run([str(launcher), "--version"], env=env, capture_output=True, text=True, check=False)
        print(result.stdout, end="")
        print(result.stderr, end="", file=sys.stderr)
        if result.returncode != 0 or expected not in result.stdout:
            print(f"::error::{wheel.name} did not report version {expected}", file=sys.stderr)
            return 1
    print(f"Wheel smoke test passed: {wheel.name} runs liter-llm {expected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
