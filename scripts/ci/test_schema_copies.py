"""Ensure the two copies of each schema registry stay byte-equal.

The crate embeds `crates/liter-llm/schemas/<name>.json` and other tooling reads
`schemas/<name>.json`. A change to one copy passes every other check.
"""

from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
REGISTRIES = ("providers.json", "catalog.json")


def copies_differ(root_copy: Path, crate_copy: Path) -> bool:
    """Return True when the two files do not hold the same bytes."""
    return root_copy.read_bytes() != crate_copy.read_bytes()


@pytest.mark.parametrize("name", REGISTRIES)
def test_the_two_copies_of_each_registry_are_equal(name: str) -> None:
    """Fail when the root copy and the crate copy differ."""
    differ = copies_differ(ROOT / "schemas" / name, ROOT / "crates" / "liter-llm" / "schemas" / name)
    assert not differ, f"schemas/{name} and crates/liter-llm/schemas/{name} differ; apply the same change to both files"


@pytest.mark.parametrize("name", REGISTRIES)
@pytest.mark.parametrize("changed_copy", ("root", "crate"))
def test_a_one_byte_difference_in_either_copy_is_detected(tmp_path: Path, name: str, changed_copy: str) -> None:
    """The check fails on a single changed byte in either copy, using the real file as the base."""
    original = (ROOT / "schemas" / name).read_bytes()
    changed = original[:-1] + bytes([original[-1] ^ 1])
    root_copy = tmp_path / "root.json"
    crate_copy = tmp_path / "crate.json"
    root_copy.write_bytes(changed if changed_copy == "root" else original)
    crate_copy.write_bytes(changed if changed_copy == "crate" else original)
    assert copies_differ(root_copy, crate_copy)
    crate_copy.write_bytes(root_copy.read_bytes())
    assert not copies_differ(root_copy, crate_copy)
