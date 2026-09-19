"""Print the `go test -run` filter that selects every test in the generated Go smoke suite."""

import re
import sys
from pathlib import Path

DEFAULT_SOURCE = Path(__file__).resolve().parents[2] / "test_apps" / "go" / "smoke_test.go"
TEST_FUNCTION = re.compile(r"^func (Test\w*)\(\w+ \*testing\.T\)", re.MULTILINE)


def smoke_test_filter(source: Path) -> str:
    """Return an anchored regex matching exactly the test functions declared in `source`."""
    names = TEST_FUNCTION.findall(source.read_text())
    if not names:
        raise SystemExit(f"No Go test functions found in {source}; the smoke task would execute zero tests")
    return f"^({'|'.join(names)})$"


def main(argv: list[str]) -> None:
    source = Path(argv[1]) if len(argv) > 1 else DEFAULT_SOURCE
    print(smoke_test_filter(source))


if __name__ == "__main__":
    main(sys.argv)
