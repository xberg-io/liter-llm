#!/usr/bin/env python3
"""
Providers documentation generator for Liter-LLM.

Reads schemas/providers.json and generates the tracked Astro docs page at
docs-site/src/content/docs/providers.md with a searchable table of all
supported LLM providers and their capabilities.

Supports --validate mode for CI (exits non-zero if docs are stale)
and --dry-run mode for preview.
"""

import argparse
import json
import logging
import sys
from pathlib import Path
from typing import Any

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger(__name__)

PROJECT_ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = PROJECT_ROOT / "schemas" / "providers.json"
OUTPUT_PATH = PROJECT_ROOT / "docs-site" / "src" / "content" / "docs" / "providers.md"
OUTPUT_REL = OUTPUT_PATH.relative_to(PROJECT_ROOT)

ENDPOINT_COLUMNS = ["chat", "embedding", "image", "audio", "moderation"]

CHECK = ":white_check_mark:"
DASH = "--"


def load_providers(schema_path: Path) -> list[dict[str, Any]]:
    """Load and return the providers list from the JSON schema."""
    with schema_path.open() as f:
        data: dict[str, Any] = json.load(f)
    return list(data["providers"])


def provider_prefix(provider: dict[str, Any]) -> str:
    """Derive the routing prefix for a provider."""
    name = provider["name"]
    return f"`{name}/`"


def endpoint_cell(provider: dict[str, Any], endpoint: str) -> str:
    """Return a checkmark or dash for a given endpoint."""
    endpoints = provider.get("endpoints", [])
    return CHECK if endpoint in endpoints else DASH


def providers_header(count: int) -> list[str]:
    """Build the page metadata and introduction."""
    return [
        "---",
        f'description: "Complete list of {count} supported LLM providers"',
        'title: "Supported Providers"',
        "---",
        "",
        (
            f"Liter-llm supports **{count} providers** out of the box. "
            "Route requests to any provider using the `provider/model` prefix convention "
            "-- for example, `openai/gpt-4o` routes to OpenAI and `anthropic/claude-3-opus` "
            "routes to Anthropic. No extra configuration is needed beyond setting the "
            "provider's API key."
        ),
        "",
    ]


def providers_table(providers: list[dict[str, Any]]) -> list[str]:
    """Build the provider capability table."""
    lines = [
        "| Provider | Prefix | Chat | Embeddings | Image | Audio | Moderation |",
        "| --- | --- | :---: | :---: | :---: | :---: | :---: |",
    ]
    for provider in providers:
        cells = [endpoint_cell(provider, endpoint) for endpoint in ENDPOINT_COLUMNS]
        lines.append(f"| {provider['display_name']} | {provider_prefix(provider)} | {' | '.join(cells)} |")
    return [*lines, "", f"*{len(providers)} providers total.*", ""]


def usage_section() -> list[str]:
    """Build the routing examples."""
    return [
        "## Usage",
        "",
        "Use any provider by prefixing the model name with the provider's routing prefix:",
        "",
        "```python",
        "from liter_llm import LiterLLM",
        "",
        "client = LiterLLM()",
        "",
        "# OpenAI",
        'response = await client.chat("openai/gpt-4o", messages=[',
        '    {"role": "user", "content": "Hello!"}',
        "])",
        "",
        "# Anthropic",
        'response = await client.chat("anthropic/claude-3-opus", messages=[',
        '    {"role": "user", "content": "Hello!"}',
        "])",
        "",
        "# Groq",
        'response = await client.chat("groq/llama3-70b", messages=[',
        '    {"role": "user", "content": "Hello!"}',
        "])",
        "```",
        "",
    ]


def custom_provider_section() -> list[str]:
    """Build the custom-provider example."""
    return [
        "## Custom Providers",
        "",
        "Any OpenAI-compatible API can be used as a custom provider by setting the base URL and API key directly:",
        "",
        "```python",
        'response = await client.chat("custom/my-model",',
        '    base_url="https://my-api.example.com/v1",',
        '    api_key="my-key",',
        "    messages=[",
        '        {"role": "user", "content": "Hello!"}',
        "    ]",
        ")",
        "```",
        "",
    ]


def provider_registry_section() -> list[str]:
    """Build the registry source link."""
    return [
        "## Provider Registry",
        "",
        (
            "The full provider registry with base URLs, auth configuration, and model "
            "mappings is available at [schemas/providers.json]"
            "(https://github.com/xberg-io/liter-llm/blob/main/schemas/providers.json)."
        ),
        "",
    ]


def generate_markdown(providers: list[dict[str, Any]]) -> str:
    """Generate the full providers.md content."""
    sorted_providers = sorted(providers, key=lambda provider: provider["display_name"].lower())
    lines = [
        *providers_header(len(sorted_providers)),
        *providers_table(sorted_providers),
        *usage_section(),
        *custom_provider_section(),
        *provider_registry_section(),
    ]

    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate providers documentation from schemas/providers.json")
    parser.add_argument(
        "--validate",
        action="store_true",
        help=f"Check if {OUTPUT_REL} matches generated output (for CI)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print generated output to stdout without writing",
    )
    args = parser.parse_args()

    if not SCHEMA_PATH.exists():
        logger.error("Schema not found: %s", SCHEMA_PATH)
        return 1

    providers = load_providers(SCHEMA_PATH)
    logger.info("Loaded %d providers from %s", len(providers), SCHEMA_PATH.name)

    content = generate_markdown(providers)

    if args.dry_run:
        print(content)
        return 0

    if args.validate:
        if not OUTPUT_PATH.exists():
            logger.error("Output file does not exist: %s", OUTPUT_PATH)
            return 1
        existing = OUTPUT_PATH.read_text()
        if existing == content:
            logger.info("%s is up-to-date", OUTPUT_REL)
            return 0
        logger.error("%s is out of date. Run 'task generate:providers-doc' to regenerate.", OUTPUT_REL)
        return 1

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(content)
    logger.info("Generated %s (%d providers)", OUTPUT_PATH.relative_to(PROJECT_ROOT), len(providers))
    return 0


if __name__ == "__main__":
    sys.exit(main())
