#!/usr/bin/env python3
"""Pricing data generator for Liter-LLM.

Fetches the unified model catalog from models.dev (maintained by the Opencode
team, MIT License) and transforms it into liter-llm's pricing.json schema.

models.dev publishes prices in USD per 1,000,000 tokens; liter-llm stores
prices in USD per token, so the script divides by 1e6.

Each upstream model is emitted under ``{provider}/{model}`` to match the
proxy's prefix-routing convention. For a small allowlist of primary providers
(openai, anthropic, google), the bare ``{model}`` key is also emitted so
callers using OpenAI-SDK-style bare names ("gpt-4o") keep resolving.

The OVERRIDES dict wins over upstream — use it for models models.dev does
not list yet, or when liter-llm needs a different value. Edit OVERRIDES, not
the generated JSON files.

Usage:
    uv run --no-sync python scripts/generate_pricing.py             # write
    uv run --no-sync python scripts/generate_pricing.py --dry-run   # stdout only
    uv run --no-sync python scripts/generate_pricing.py --validate  # CI check
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
import urllib.request
from pathlib import Path
from typing import Any

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger(__name__)

PROJECT_ROOT = Path(__file__).resolve().parent.parent
ROOT_OUTPUT = PROJECT_ROOT / "schemas" / "pricing.json"
CRATE_OUTPUT = PROJECT_ROOT / "crates" / "liter-llm" / "schemas" / "pricing.json"
OUTPUTS = (ROOT_OUTPUT, CRATE_OUTPUT)

MODELS_DEV_URL = "https://models.dev/api.json"
TOKENS_PER_UNIT = 1_000_000

PRIMARY_PROVIDERS = ("openai", "anthropic", "google")

OVERRIDES: dict[str, dict[str, float]] = {}

HEADER_COMMENT = (
    "Model pricing data generated from models.dev (MIT License, Opencode team) via "
    "scripts/generate_pricing.py. Prices in USD per token. Run `task generate:pricing` to refresh."
)


USER_AGENT = "liter-llm-pricing-generator/1.0 (+https://github.com/xberg-io/liter-llm)"


def fetch_catalog(url: str) -> dict[str, Any]:
    """Fetch the upstream model catalog from an HTTPS URL."""
    if not url.startswith("https://"):
        raise ValueError(f"Refusing to fetch non-HTTPS URL: {url}")
    logger.info("Fetching %s", url)
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})  # noqa: S310
    with urllib.request.urlopen(request, timeout=30) as response:  # noqa: S310
        data: dict[str, Any] = json.load(response)
    return data


def transform(catalog: dict[str, Any]) -> dict[str, dict[str, float]]:
    """Transform the upstream catalog into liter-llm pricing records."""
    models: dict[str, dict[str, float]] = {}
    skipped = 0
    for provider_id, provider in catalog.items():
        for model_id, model in provider.get("models", {}).items():
            cost = model.get("cost")
            if not cost or "input" not in cost:
                skipped += 1
                continue
            entry: dict[str, float] = {
                "input_cost_per_token": float(cost["input"]) / TOKENS_PER_UNIT,
                "output_cost_per_token": float(cost.get("output", 0.0)) / TOKENS_PER_UNIT,
            }
            if "cache_read" in cost:
                entry["cache_read_input_token_cost"] = float(cost["cache_read"]) / TOKENS_PER_UNIT
            if "cache_write" in cost:
                entry["cache_creation_input_token_cost"] = float(cost["cache_write"]) / TOKENS_PER_UNIT
            models[f"{provider_id}/{model_id}"] = entry
            if provider_id in PRIMARY_PROVIDERS:
                models.setdefault(model_id, entry)
    logger.info("Imported %d entries from upstream, skipped %d models without pricing", len(models), skipped)
    return models


def apply_overrides(models: dict[str, dict[str, float]]) -> dict[str, dict[str, float]]:
    """Apply local pricing overrides to transformed upstream model records."""
    for key, entry in OVERRIDES.items():
        models[key] = dict(entry)
    return models


def format_cost(value: float) -> str:
    """Format a per-token cost as a fixed-point decimal — avoids 2.5e-06 noise in diffs."""
    if value == 0:
        return "0.0"
    text = f"{value:.15f}".rstrip("0").rstrip(".")
    return text if "." in text else f"{text}.0"


def render(models: dict[str, dict[str, float]]) -> str:
    """Render pricing records as deterministic JSON text."""
    sorted_items = sorted(models.items())
    lines = ["{", f'\t"$comment": {json.dumps(HEADER_COMMENT)},', '\t"models": {']
    for i, (key, entry) in enumerate(sorted_items):
        suffix = "," if i < len(sorted_items) - 1 else ""
        body = [
            f'\t\t\t"input_cost_per_token": {format_cost(entry["input_cost_per_token"])}',
            f'\t\t\t"output_cost_per_token": {format_cost(entry["output_cost_per_token"])}',
        ]
        if "cache_read_input_token_cost" in entry:
            body.append(f'\t\t\t"cache_read_input_token_cost": {format_cost(entry["cache_read_input_token_cost"])}')
        if "cache_creation_input_token_cost" in entry:
            body.append(
                f'\t\t\t"cache_creation_input_token_cost": {format_cost(entry["cache_creation_input_token_cost"])}'
            )
        body_joined = ",\n".join(body)
        lines.append(f"\t\t{json.dumps(key)}: {{\n{body_joined}\n\t\t}}{suffix}")
    lines.extend(["\t}", "}", ""])
    rendered = "\n".join(lines)
    json.loads(rendered)
    return rendered


def main() -> int:
    """Run the pricing generator CLI."""
    parser = argparse.ArgumentParser(description="Generate pricing.json from models.dev")
    parser.add_argument("--dry-run", action="store_true", help="Print to stdout without writing")
    parser.add_argument("--validate", action="store_true", help="CI: fail non-zero if generated output drifts")
    parser.add_argument("--url", default=MODELS_DEV_URL, help="Source URL (default: %(default)s)")
    args = parser.parse_args()

    catalog = fetch_catalog(args.url)
    models = apply_overrides(transform(catalog))
    content = render(models)

    if args.dry_run:
        sys.stdout.write(content)
        return 0

    if args.validate:
        stale = [p for p in OUTPUTS if (p.read_text() if p.exists() else "") != content]
        if stale:
            for path in stale:
                logger.error("%s is out of date — run `task generate:pricing`", path.relative_to(PROJECT_ROOT))
            return 1
        logger.info("All pricing files are up to date (%d entries)", len(models))
        return 0

    for path in OUTPUTS:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        logger.info("Wrote %s (%d entries)", path.relative_to(PROJECT_ROOT), len(models))
    return 0


if __name__ == "__main__":
    sys.exit(main())
