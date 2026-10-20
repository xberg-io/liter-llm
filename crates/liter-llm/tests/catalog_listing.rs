//! The built-in catalog listing, reached through the crate's public path.

use liter_llm::{CatalogModel, exact_model_info, list_catalog_models};

#[test]
fn the_crate_root_lists_every_provider_without_a_client() {
    let all: Vec<CatalogModel> = list_catalog_models(None);
    assert!(
        all.len() > 100,
        "the embedded catalog holds many models, got {}",
        all.len()
    );
    assert!(all.windows(2).all(|pair| pair[0].key < pair[1].key), "sorted by key");
    for entry in all.iter().take(25) {
        let info = exact_model_info(&entry.key).unwrap_or_else(|| panic!("{} must resolve exactly", entry.key));
        assert_eq!(info.max_tokens, entry.max_tokens, "context of {}", entry.key);
    }
}

#[test]
fn the_crate_root_filters_by_exact_provider_id() {
    let anthropic = list_catalog_models(Some("anthropic"));
    assert!(!anthropic.is_empty());
    assert!(anthropic.iter().all(|m| m.provider == "anthropic"));
    assert!(list_catalog_models(Some("Anthropic")).is_empty());
    assert!(list_catalog_models(Some("no-such-provider")).is_empty());
}

#[test]
fn an_empty_provider_lists_every_provider() {
    let all = list_catalog_models(None);
    assert!(!all.is_empty());
    assert_eq!(list_catalog_models(Some("")), all);
}
