"""Print the poly release tag mise.toml pins, for workflows that install poly without mise."""

import re
import sys
import tomllib
from pathlib import Path

TOOL = "github:Goldziher/poly"
# ~keep A release tag: `latest`, a partial version or a table pin would install an unpinned poly.
RELEASE = re.compile(r"v?\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?")


def poly_version(mise_toml: Path) -> str:
    with mise_toml.open("rb") as config:
        version = tomllib.load(config).get("tools", {}).get(TOOL)
    if version is None:
        raise ValueError(f"{mise_toml} pins no {TOOL}")
    if not isinstance(version, str) or not RELEASE.fullmatch(version):
        raise ValueError(f"{mise_toml} pins {TOOL} to {version!r}, not a release version such as 0.32.5")
    return f"v{version.removeprefix('v')}"


def main() -> None:
    try:
        print(poly_version(Path(sys.argv[1] if len(sys.argv) > 1 else "mise.toml")))
    except ValueError as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
