"""Ensure every maintained registry consumer is checked during version synchronization."""

from pathlib import Path

import pytest
import tomllib
from check_consumer_versions import OWNED_PINS, PINS, expected_pin, replace_pin, sync_owned, verify_all, verify_pin

ROOT = Path(__file__).resolve().parents[2]
CURRENT_VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
EXTRA_PINS = ("dart/pubspec.yaml", "kotlin_android/build.gradle.kts")


@pytest.fixture
def consumers(tmp_path: Path) -> Path:
    """Use real manifest shapes in an isolated tree, without changing repository files."""
    for relative in (*PINS, *EXTRA_PINS):
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text((ROOT / "test_apps" / relative).read_text())
    sync_owned(tmp_path, CURRENT_VERSION)
    return tmp_path


def test_every_configured_registry_target_has_a_version_guard() -> None:
    """Adding a maintained language must also add a live coordinate check."""
    manifest = tomllib.loads((ROOT / "alef.toml").read_text())
    crate = next(crate for crate in manifest["crates"] if crate["name"] == "liter-llm")
    expected = set(crate["e2e"]["languages"])
    aliases = {"swift_e2e": "swift"}
    actual = {aliases.get(path.split("/")[0], path.split("/")[0]) for path in (*PINS, *EXTRA_PINS)}
    assert actual == expected


def test_all_user_owned_manifest_pins_are_synchronized() -> None:
    """Create-once ownership must not silently exclude a maintained dependency."""
    manifest = tomllib.loads((ROOT / "alef.toml").read_text())
    maintained = {f"test_apps/{path}" for path in (*PINS, *EXTRA_PINS)}
    expected = set(manifest["workspace"]["ownership"]["user_owned"]) & maintained
    assert {f"test_apps/{path}" for path in (*OWNED_PINS, *EXTRA_PINS)} == expected


def test_current_manifests_pass_and_sync_is_idempotent(consumers: Path) -> None:
    """A second synchronization preserves all manifest content."""
    before = {path: (consumers / path).read_text() for path in (*PINS, *EXTRA_PINS)}
    verify_all(consumers, CURRENT_VERSION)
    sync_owned(consumers, CURRENT_VERSION)
    assert {path: (consumers / path).read_text() for path in before} == before


@pytest.mark.parametrize("relative", [*PINS, *EXTRA_PINS])
def test_each_stale_consumer_is_rejected(consumers: Path, relative: str) -> None:
    """A stale dependency in any maintained consumer must fail the actual-file gate."""
    path = consumers / relative
    content = path.read_text()
    assert CURRENT_VERSION in content
    path.write_text(content.replace(CURRENT_VERSION, "1.0.0"))
    with pytest.raises(SystemExit):
        verify_all(consumers, CURRENT_VERSION)


@pytest.mark.parametrize("content", ["", '{:liter_llm, "1.16.0"}\n{:liter_llm, "1.18.4"}\n'])
def test_owned_sync_rejects_missing_or_duplicate_coordinates(content: str) -> None:
    """Ambiguous owned files must fail instead of silently certifying a version bump."""
    with pytest.raises(SystemExit, match="exactly one"):
        replace_pin(content, PINS["elixir/mix.exs"], CURRENT_VERSION)


def test_go_major_bump_updates_module_suffix_and_preserves_other_dependencies() -> None:
    """Go semantic import versions must advance with the required release."""
    old = "github.com/xberg-io/liter-llm/packages/go/v2 v2.0.0"
    new = "github.com/xberg-io/liter-llm/packages/go/v3 v3.0.0"
    original = f"module example.test\n\nrequire (\n\t{old}\n\texample.org/other v1.5.0\n)\n"
    assert replace_pin(original, PINS["go/go.mod"], expected_pin("go/go.mod", "3.0.0")) == original.replace(old, new)


def test_homebrew_version_guard_reads_the_executed_bats_suite() -> None:
    """The launcher delegates to Bats, so its suite owns the effective version pin."""
    assert "homebrew/run_tests.sh" not in PINS
    assert "homebrew/homebrew.bats" in PINS
    suite = (ROOT / "test_apps/homebrew/homebrew.bats").read_text()
    verify_pin(suite, PINS["homebrew/homebrew.bats"], CURRENT_VERSION, "homebrew/homebrew.bats")
