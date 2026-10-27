"""Print the CHANGELOG.md entry of one version, without its heading."""

import re
import sys
from pathlib import Path


def changelog_entry(changelog: str, version: str) -> str:
    lines = changelog.splitlines()
    heading = re.compile(rf"^## \[{re.escape(version)}\](?:\s|$)")
    start = next((i for i, line in enumerate(lines) if heading.match(line)), None)
    if start is None:
        raise ValueError(f"CHANGELOG.md has no entry for version {version}")
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## [")), len(lines))
    body = "\n".join(lines[start + 1 : end]).strip()
    if not body:
        raise ValueError(f"CHANGELOG.md entry for version {version} is empty")
    return body


def main() -> None:
    changelog_path, version = sys.argv[1], sys.argv[2]
    try:
        print(changelog_entry(Path(changelog_path).read_text(encoding="utf-8"), version))
    except ValueError as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
