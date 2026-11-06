"""Print the poly release tag mise.toml pins, for workflows that install poly without mise."""

import sys
import tomllib
from pathlib import Path

TOOL = "github:Goldziher/poly"


def poly_version(mise_toml: Path) -> str:
    with mise_toml.open("rb") as config:
        version = tomllib.load(config).get("tools", {}).get(TOOL)
    if not isinstance(version, str) or not version:
        raise ValueError(f"{mise_toml} pins no {TOOL}")
    return f"v{version.removeprefix('v')}"


def main() -> None:
    try:
        print(poly_version(Path(sys.argv[1] if len(sys.argv) > 1 else "mise.toml")))
    except ValueError as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
