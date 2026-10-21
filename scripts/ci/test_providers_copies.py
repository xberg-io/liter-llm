"""Ensure the two copies of the provider registry stay byte-equal.

The crate embeds `crates/liter-llm/schemas/providers.json` and the docs generator reads
`schemas/providers.json`. An entry added to one copy only changes no other check.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ROOT_COPY = ROOT / "schemas" / "providers.json"
CRATE_COPY = ROOT / "crates" / "liter-llm" / "schemas" / "providers.json"


def test_the_two_copies_of_providers_json_are_equal() -> None:
    """Fail when the root copy and the crate copy differ."""
    assert ROOT_COPY.read_bytes() == CRATE_COPY.read_bytes(), (
        "schemas/providers.json and crates/liter-llm/schemas/providers.json differ; "
        "apply the same change to both files"
    )
