"""The CLI proxy packages and the MCP Registry entry must move with the release, and drift must fail."""

import json
import shutil
from pathlib import Path

import pytest
from check_cli_proxy_versions import NPM_PACKAGE, SERVER_NAME, core_version, drift, pypi_spelling, read_versions, sync

ROOT = Path(__file__).resolve().parents[2]
FILES = (
    "Cargo.toml",
    "server.json",
    "cli-proxy/npm/package.json",
    "cli-proxy/pypi/pyproject.toml",
    "cli-proxy/pypi/liter_llm_cli/__init__.py",
)


@pytest.fixture
def tree(tmp_path: Path) -> Path:
    """Copy the real manifests into an isolated tree so mutations never touch the repository."""
    for relative in FILES:
        target = tmp_path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(ROOT / relative, target)
    return tmp_path


def test_repository_proxies_match_the_core_version() -> None:
    assert drift(core_version()) == []


def test_every_tracked_version_is_read() -> None:
    assert len(read_versions(ROOT)) == 5


def test_drift_in_the_npm_manifest_is_reported(tree: Path) -> None:
    path = tree / "cli-proxy/npm/package.json"
    manifest = json.loads(path.read_text())
    manifest["version"] = "0.0.1"
    path.write_text(json.dumps(manifest))
    assert any("package.json version -> 0.0.1" in line for line in drift(core_version(tree), tree))


def test_a_stale_platform_package_pin_is_reported(tree: Path) -> None:
    path = tree / "cli-proxy/npm/package.json"
    manifest = json.loads(path.read_text())
    name = next(iter(manifest["optionalDependencies"]))
    manifest["optionalDependencies"][name] = "0.0.1"
    path.write_text(json.dumps(manifest))
    assert any(f"optionalDependencies {name} -> 0.0.1" in line for line in drift(core_version(tree), tree))


def test_missing_platform_packages_are_reported(tree: Path) -> None:
    path = tree / "cli-proxy/npm/package.json"
    manifest = json.loads(path.read_text())
    del manifest["optionalDependencies"]
    path.write_text(json.dumps(manifest))
    assert any("no optionalDependencies" in line for line in drift(core_version(tree), tree))


def test_drift_in_the_pypi_manifest_is_reported(tree: Path) -> None:
    path = tree / "cli-proxy/pypi/pyproject.toml"
    path.write_text(path.read_text().replace(f'version = "{core_version(tree)}"', 'version = "0.1.0"'))
    assert any("pyproject.toml version -> 0.1.0" in line for line in drift(core_version(tree), tree))


def test_a_stale_registry_package_version_is_reported(tree: Path) -> None:
    path = tree / "server.json"
    server = json.loads(path.read_text())
    server["packages"][0]["version"] = "1.0.0"
    path.write_text(json.dumps(server))
    assert any("server.json packages[npm].version -> 1.0.0" in line for line in drift(core_version(tree), tree))


def test_a_mismatched_mcp_name_is_reported(tree: Path) -> None:
    path = tree / "cli-proxy/npm/package.json"
    manifest = json.loads(path.read_text())
    manifest["mcpName"] = "io.github.someone/else"
    path.write_text(json.dumps(manifest))
    assert any("mcpName" in line for line in drift(core_version(tree), tree))


def test_sync_rewrites_the_files_alef_does_not_own(tree: Path) -> None:
    sync("9.9.9", tree)
    versions = read_versions(tree)
    assert versions["server.json version"] == "9.9.9"
    assert versions["server.json packages[npm].version"] == "9.9.9"
    assert versions["cli-proxy/pypi/liter_llm_cli/__init__.py __version__"] == "9.9.9"
    pins = json.loads((tree / "cli-proxy/npm/package.json").read_text())["optionalDependencies"]
    assert set(pins.values()) == {"9.9.9"}


def test_registry_entry_names_the_published_npm_package() -> None:
    server = json.loads((ROOT / "server.json").read_text())
    assert server["name"] == SERVER_NAME
    assert [(p["registryType"], p["identifier"]) for p in server["packages"]] == [("npm", NPM_PACKAGE)]


def test_prerelease_uses_the_pep_440_spelling_in_pyproject_only(tree: Path) -> None:
    """alef writes 2.8.0rc1 into pyproject; the release tag and npm/registry keep 2.8.0-rc.1."""
    sync("2.8.0-rc.1", tree)
    npm = tree / "cli-proxy/npm/package.json"
    manifest = json.loads(npm.read_text())
    manifest["version"] = "2.8.0-rc.1"
    npm.write_text(json.dumps(manifest))
    pyproject = tree / "cli-proxy/pypi/pyproject.toml"
    pyproject.write_text(pyproject.read_text().replace(f'version = "{core_version(tree)}"', 'version = "2.8.0rc1"'))
    assert drift("2.8.0-rc.1", tree) == []
    pyproject.write_text(pyproject.read_text().replace("2.8.0rc1", "2.8.0-rc.1"))
    assert len(drift("2.8.0-rc.1", tree)) == 1


@pytest.mark.parametrize(
    ("version", "expected"),
    [("2.8.0", "2.8.0"), ("2.8.0-rc.1", "2.8.0rc1"), ("2.8.0-alpha.2", "2.8.0a2"), ("2.8.0-beta.3", "2.8.0b3")],
)
def test_pypi_spelling(version: str, expected: str) -> None:
    assert pypi_spelling(version) == expected
