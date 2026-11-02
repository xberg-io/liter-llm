"""Check Kotlin's registry dependency and AAR verification against the release version."""

import re
import sys
import tomllib
from pathlib import Path

COORDINATE = "io.xberg.literllm:liter-llm-android"
EXPECTED_COORDINATES = 2


def without_comments(content: str) -> str:
    """Remove nested Kotlin comments without interpreting delimiters inside strings."""
    result = []
    index = 0
    depth = 0
    while index < len(content):
        if content.startswith("/*", index):
            depth += 1
            result.append("  ")
            index += 2
        elif depth and content.startswith("*/", index):
            depth -= 1
            result.append("  ")
            index += 2
        elif depth:
            result.append("\n" if content[index] == "\n" else " ")
            index += 1
        elif content.startswith("//", index):
            end = content.find("\n", index)
            end = len(content) if end < 0 else end
            result.append(" " * (end - index))
            index = end
        elif content[index] == '"':
            match = re.match(r'"""[\s\S]*?"""|"(?:\\.|[^"\\])*"', content[index:])
            if match is None:
                raise SystemExit("Kotlin consumer has an unterminated string")
            result.append(match.group())
            index += match.end()
        else:
            result.append(content[index])
            index += 1
    if depth:
        raise SystemExit("Kotlin consumer has an unterminated block comment")
    return "".join(result)


def verify_coordinates(content: str, version: str) -> None:
    """Reject stale, missing, or duplicate release coordinates."""
    content = without_comments(content)
    coordinate = re.escape(COORDINATE)
    versions = re.findall(rf'{coordinate}:([^"\s]+)', content)
    if versions != [version] * EXPECTED_COORDINATES:
        raise SystemExit(f"Kotlin consumer must declare exactly two {COORDINATE}:{version} pins; found {versions}")
    for declaration in (r"implementation\(", r"val\s+aarCoord\s*=\s*"):
        matches = re.findall(rf'{declaration}"{coordinate}:([^"\s]+)"', content)
        if matches != [version]:
            raise SystemExit(f"Kotlin consumer declaration {declaration} must select {version}; found {matches}")


def synchronize_coordinates(content: str, version: str) -> str:
    """Update both live coordinates while preserving comments and unrelated versions."""
    pattern = rf'{re.escape(COORDINATE)}:([^"\s]+)'
    matches = list(re.finditer(pattern, without_comments(content)))
    if len(matches) != EXPECTED_COORDINATES:
        raise SystemExit("Kotlin consumer must declare exactly two live coordinates")
    for match in reversed(matches):
        content = content[: match.start(1)] + version + content[match.end(1) :]
    verify_coordinates(content, version)
    return content


def main() -> None:
    """Verify the source pins without resolving an unpublished Maven package."""
    root = Path(__file__).resolve().parents[2]
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    version = manifest["workspace"]["package"]["version"]
    if not isinstance(version, str) or not version:
        raise SystemExit("Kotlin consumer cannot be checked without a canonical Cargo version")
    path = root / "test_apps/kotlin_android/build.gradle.kts"
    if "--sync" in sys.argv:
        path.write_text(synchronize_coordinates(path.read_text(), version))
    verify_coordinates(path.read_text(), version)
    print(f"Kotlin consumer: {EXPECTED_COORDINATES} coordinates match {version}; registry resolution is separate")


if __name__ == "__main__":
    main()
