//! Deterministic, poly-fmt-clean JSON rendering for `catalog.json`.
//!
//! Costs are rendered as fixed-point decimals (never `2.5e-06`), keys follow
//! the fixed order from the `catalog.json` schema (see the crate-level
//! docs), and indentation is two spaces (not tabs) so `poly fmt --check .`
//! passes on the output without a reformatting pass.

use std::fmt::Write as _;

use crate::Provenance;
use crate::error::CatalogGenError;
use crate::transform::{CatalogDocument, CatalogModel, CatalogPricing, CatalogPricingTier, CatalogProvider};

const SCHEMA_VERSION: u32 = 1;

/// Render the full `catalog.json` document: `$provenance`,
/// `$schema_version`, and the nested `providers` map.
pub fn render_document(catalog: &CatalogDocument, provenance: &Provenance) -> Result<String, CatalogGenError> {
    let mut out = String::new();
    out.push_str("{\n");
    render_provenance(&mut out, provenance);
    let _ = writeln!(out, "  \"$schema_version\": {SCHEMA_VERSION},");
    out.push_str("  \"providers\": {\n");

    let count = catalog.len();
    for (index, (provider_id, provider)) in catalog.iter().enumerate() {
        render_provider(&mut out, provider_id, provider, index + 1 == count);
    }

    out.push_str("  }\n");
    out.push_str("}\n");

    serde_json::from_str::<serde_json::Value>(&out).map_err(CatalogGenError::RoundTrip)?;

    Ok(out)
}

fn render_provenance(out: &mut String, provenance: &Provenance) {
    out.push_str("  \"$provenance\": {\n");
    let _ = writeln!(out, "    \"source\": {},", json_string(&provenance.source));
    let _ = writeln!(
        out,
        "    \"source_sha256\": {},",
        json_string(&provenance.source_sha256)
    );
    let _ = writeln!(out, "    \"fetched\": {},", json_string(&provenance.fetched));
    let _ = writeln!(
        out,
        "    \"library_version\": {}",
        json_string(&provenance.library_version)
    );
    out.push_str("  },\n");
}

fn render_provider(out: &mut String, provider_id: &str, provider: &CatalogProvider, is_last: bool) {
    let _ = writeln!(out, "    {}: {{", json_string(provider_id));
    let _ = writeln!(out, "      \"name\": {},", json_string(&provider.name));
    let _ = writeln!(out, "      \"env\": {},", json_string_array(&provider.env));
    let _ = writeln!(out, "      \"doc\": {},", json_string(&provider.doc));
    if let Some(api) = &provider.api {
        let _ = writeln!(out, "      \"api\": {},", json_string(api));
    }
    let _ = writeln!(out, "      \"npm\": {},", json_string(&provider.npm));
    out.push_str("      \"models\": {\n");

    let count = provider.models.len();
    for (index, (model_id, model)) in provider.models.iter().enumerate() {
        render_model(out, model_id, model, index + 1 == count);
    }

    out.push_str("      }\n");
    let suffix = if is_last { "" } else { "," };
    let _ = writeln!(out, "    }}{suffix}");
}

fn render_model(out: &mut String, model_id: &str, model: &CatalogModel, is_last: bool) {
    let _ = writeln!(out, "        {}: {{", json_string(model_id));

    let mut fields: Vec<String> = Vec::new();
    fields.push(format!("          \"id\": {}", json_string(&model.id)));
    fields.push(format!("          \"name\": {}", json_string(&model.name)));
    if let Some(pricing) = &model.pricing {
        fields.push(pricing_field(pricing));
    }
    fields.push(limit_field(model));
    fields.push(modalities_field(model));
    if let Some(mode) = &model.mode {
        fields.push(format!("          \"mode\": {}", json_string(mode)));
    }
    fields.push(capabilities_field(model));
    if let Some(shape) = &model.shape {
        fields.push(format!("          \"shape\": {}", json_string(shape)));
    }
    if let Some(family) = &model.family {
        fields.push(format!("          \"family\": {}", json_string(family)));
    }
    if let Some(knowledge) = &model.knowledge {
        fields.push(format!("          \"knowledge\": {}", json_string(knowledge)));
    }
    fields.push(format!(
        "          \"release_date\": {}",
        json_string(&model.release_date)
    ));
    fields.push(format!(
        "          \"last_updated\": {}",
        json_string(&model.last_updated)
    ));

    out.push_str(&fields.join(",\n"));
    out.push('\n');
    let suffix = if is_last { "" } else { "," };
    let _ = writeln!(out, "        }}{suffix}");
}

fn pricing_field(pricing: &CatalogPricing) -> String {
    let mut fields: Vec<String> = Vec::new();
    fields.push(cost_field_indent(
        "input_cost_per_token",
        pricing.input_cost_per_token,
        12,
    ));
    fields.push(cost_field_indent(
        "output_cost_per_token",
        pricing.output_cost_per_token,
        12,
    ));
    push_optional_cost_indent(
        &mut fields,
        "cache_read_input_token_cost",
        pricing.cache_read_input_token_cost,
        12,
    );
    push_optional_cost_indent(
        &mut fields,
        "cache_creation_input_token_cost",
        pricing.cache_creation_input_token_cost,
        12,
    );
    push_optional_cost_indent(
        &mut fields,
        "input_cost_per_audio_token",
        pricing.input_cost_per_audio_token,
        12,
    );
    push_optional_cost_indent(
        &mut fields,
        "output_cost_per_audio_token",
        pricing.output_cost_per_audio_token,
        12,
    );
    push_optional_cost_indent(
        &mut fields,
        "output_cost_per_reasoning_token",
        pricing.output_cost_per_reasoning_token,
        12,
    );
    if !pricing.tiers.is_empty() {
        fields.push(tiers_field(&pricing.tiers));
    }

    let mut out = String::from("          \"pricing\": {\n");
    out.push_str(&fields.join(",\n"));
    out.push('\n');
    out.push_str("          }");
    out
}

fn tiers_field(tiers: &[CatalogPricingTier]) -> String {
    let mut out = String::from("            \"tiers\": [\n");
    let count = tiers.len();
    for (index, tier) in tiers.iter().enumerate() {
        out.push_str("              {\n");
        let mut fields: Vec<String> = vec![format!(
            "                \"min_context_tokens\": {}",
            tier.min_context_tokens
        )];
        fields.push(cost_field_indent("input_cost_per_token", tier.input_cost_per_token, 16));
        fields.push(cost_field_indent(
            "output_cost_per_token",
            tier.output_cost_per_token,
            16,
        ));
        push_optional_cost_indent(
            &mut fields,
            "cache_read_input_token_cost",
            tier.cache_read_input_token_cost,
            16,
        );
        push_optional_cost_indent(
            &mut fields,
            "cache_creation_input_token_cost",
            tier.cache_creation_input_token_cost,
            16,
        );
        push_optional_cost_indent(
            &mut fields,
            "input_cost_per_audio_token",
            tier.input_cost_per_audio_token,
            16,
        );
        push_optional_cost_indent(
            &mut fields,
            "output_cost_per_audio_token",
            tier.output_cost_per_audio_token,
            16,
        );
        push_optional_cost_indent(
            &mut fields,
            "output_cost_per_reasoning_token",
            tier.output_cost_per_reasoning_token,
            16,
        );
        out.push_str(&fields.join(",\n"));
        out.push('\n');
        let suffix = if index + 1 == count { "" } else { "," };
        out.push_str(&format!("              }}{suffix}\n"));
    }
    out.push_str("            ]");
    out
}

fn limit_field(model: &CatalogModel) -> String {
    let mut fields: Vec<String> = vec![format!("            \"context\": {}", model.limit.context)];
    if let Some(input) = model.limit.input {
        fields.push(format!("            \"input\": {input}"));
    }
    fields.push(format!("            \"output\": {}", model.limit.output));

    let mut out = String::from("          \"limit\": {\n");
    out.push_str(&fields.join(",\n"));
    out.push('\n');
    out.push_str("          }");
    out
}

fn modalities_field(model: &CatalogModel) -> String {
    let mut out = String::from("          \"modalities\": {\n");
    let _ = writeln!(
        out,
        "            \"input\": {},",
        json_string_array(&model.modalities.input)
    );
    let _ = writeln!(
        out,
        "            \"output\": {}",
        json_string_array(&model.modalities.output)
    );
    out.push_str("          }");
    out
}

fn capabilities_field(model: &CatalogModel) -> String {
    let capabilities = &model.capabilities;
    let fields = [
        ("vision", capabilities.vision),
        ("function_calling", capabilities.function_calling),
        ("reasoning", capabilities.reasoning),
        ("structured_output", capabilities.structured_output),
        ("audio_input", capabilities.audio_input),
        ("audio_output", capabilities.audio_output),
        ("prompt_caching", capabilities.prompt_caching),
        ("attachment", capabilities.attachment),
        ("open_weights", capabilities.open_weights),
    ];
    let rendered: Vec<String> = fields
        .iter()
        .map(|(name, value)| format!("            \"{name}\": {value}"))
        .collect();

    let mut out = String::from("          \"capabilities\": {\n");
    out.push_str(&rendered.join(",\n"));
    out.push('\n');
    out.push_str("          }");
    out
}

fn cost_field_indent(name: &str, value: f64, indent: usize) -> String {
    format!("{}\"{name}\": {}", " ".repeat(indent), format_cost(value))
}

fn push_optional_cost_indent(fields: &mut Vec<String>, name: &str, value: Option<f64>, indent: usize) {
    if let Some(value) = value {
        fields.push(cost_field_indent(name, value, indent));
    }
}

/// Format a per-token cost as a fixed-point decimal (never emits `2.5e-06`).
pub fn format_cost(value: f64) -> String {
    if value == 0.0 {
        return "0.0".to_string();
    }
    let text = format!("{value:.15}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_string()
    } else {
        format!("{trimmed}.0")
    }
}

/// JSON-escape and quote a string via `serde_json`, keeping escaping rules
/// identical to the rest of the JSON we emit.
fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

/// Render a string array as a single-line JSON array, e.g. `["a", "b"]`.
fn json_string_array(values: &[String]) -> String {
    let rendered: Vec<String> = values.iter().map(|value| json_string(value)).collect();
    format!("[{}]", rendered.join(", "))
}
