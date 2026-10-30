---
status: accepted
date: 2026-07-19
decision-makers: ["Na'aman Hirschfeld"]
---

# Model catalog generated from models.dev

## Context and Problem Statement

Cost estimation and budget enforcement need per-model prices, and callers need per-model metadata such as context
window and capabilities. The embedded pricing file started with 35 hand-curated entries. A Python script then
generated it from the models.dev catalog (791d681ca, 2026-04-26, closing #48), raising coverage to 4,219 models and
"closing the data-staleness gap for cost tracking and budget enforcement". On 2026-07-19 (b8c39e638) "the Python
pricing/providers scripts" were replaced by a Rust generator that strictly validates the models.dev `api.json` and writes `catalog.json`
(renamed from `pricing.json`) with a `$provenance` block, together with sync, publish and verify workflows.

## Considered Options

* A Rust generator crate, `liter-llm-catalog-gen`, that fetches and strictly validates models.dev and writes the
  embedded `catalog.json`, refreshed daily by CI
* A Python script that fetches models.dev and writes `pricing.json` (791d681ca, replaced in b8c39e638)
* A hand-curated `pricing.json` (replaced in 791d681ca)

## Decision Outcome

Chosen option: "A Rust generator crate, `liter-llm-catalog-gen`, that fetches and strictly validates models.dev and
writes the embedded `catalog.json`, refreshed daily by CI", because models.dev covers far more models than a
hand-kept list, strict validation rejects an upstream schema change instead of writing a wrong catalog, and a daily
job keeps the committed catalog current.

The generator writes both `schemas/catalog.json` and `crates/liter-llm/schemas/catalog.json`, and the core crate
embeds the crate copy. Both copies are generated and are never hand-edited. An opt-in runtime refresh can overlay a
downloaded catalog on the embedded one; it is off by default, so an air-gapped build keeps working.

### Consequences

* Good, because the catalog follows models.dev within a day, through an automated pull request.
* Good, because a malformed upstream catalog fails the generator instead of shipping.
* Bad, because the catalog depends on a third-party source, and `--validate` needs the network, so the verify
  workflow is manual-only.

### Confirmation

* `crates/liter-llm-catalog-gen/src/lib.rs:30` sets the default source `https://models.dev/api.json`.
* `crates/liter-llm/src/cost.rs:27` embeds `../schemas/catalog.json`.
* `mise.toml:153` (`generate:catalog`) and `mise.toml:157` (`generate:catalog:check`) run the generator.
* `.github/workflows/sync-catalog.yml:5` schedules the daily refresh, which opens a pull request on drift.
* `scripts/ci/test_schema_copies.py:12` keeps the two `catalog.json` copies byte-equal.
* The `model-catalog` rule in `.ai-rulez/rules/model-catalog.md` forbids hand-editing either copy.
