use std::borrow::Cow;

use serde_json::Value;

use crate::error::Result;
use crate::provider::Provider;
use crate::provider::rerank_shape::reshape_rerank_reply;

/// Voyage AI provider.
///
/// Requests and embedding replies are OpenAI-compatible. The rerank reply is not:
/// Voyage lists the scored documents under `data`, where the canonical shape uses
/// `results`, and returns the document text of each hit as a plain string.
/// [`Provider::transform_rerank_response`] maps both onto the canonical shape.
pub struct VoyageProvider;

impl Provider for VoyageProvider {
    fn name(&self) -> &str {
        "voyage"
    }

    fn base_url(&self) -> &str {
        "https://api.voyageai.com/v1"
    }

    fn env_var(&self) -> Option<&str> {
        Some("VOYAGE_API_KEY")
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("voyage/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("voyage/").unwrap_or(model)
    }

    /// Move the `data` array to `results`, and wrap string documents as `{ "text": ... }`.
    ///
    /// The mapping lives in [`crate::provider::rerank_shape`].
    fn transform_rerank_response(&self, body: &mut Value) -> Result<()> {
        reshape_rerank_reply(self.name(), body);
        Ok(())
    }

    /// Voyage documents `top_k` for the number of results and does not list `top_n`.
    fn transform_rerank_request(&self, body: &mut Value) -> Result<()> {
        if let Some(obj) = body.as_object_mut()
            && let Some(top_n) = obj.remove("top_n")
        {
            obj.insert("top_k".to_owned(), top_n);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_voyage_name_and_base_url_match_registry() {
        let provider = VoyageProvider;
        assert_eq!(provider.name(), "voyage");
        let registry_entry = crate::provider::all_providers()
            .expect("registry should load")
            .iter()
            .find(|p| p.name == "voyage")
            .expect("voyage should be in the registry");
        assert_eq!(Some(provider.base_url()), registry_entry.base_url.as_deref());
        assert_eq!(
            provider.env_var(),
            registry_entry.auth.as_ref().and_then(|a| a.env_var.as_deref())
        );
    }

    #[test]
    fn test_voyage_auth_header() {
        let (name, value) = VoyageProvider.auth_header("test-key").expect("auth header");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer test-key");
    }

    #[test]
    fn test_voyage_matches_and_strips_prefix() {
        let provider = VoyageProvider;
        assert!(provider.matches_model("voyage/rerank-2"));
        assert!(!provider.matches_model("rerank-2"));
        assert!(!provider.matches_model("cohere/rerank-v3.5"));
        assert_eq!(provider.strip_model_prefix("voyage/rerank-2"), "rerank-2");
        assert_eq!(provider.strip_model_prefix("rerank-2"), "rerank-2");
    }

    #[test]
    fn test_voyage_rerank_reply_data_becomes_results() {
        let mut body = json!({
            "object": "list",
            "data": [{"index": 2, "relevance_score": 0.9}, {"index": 0, "relevance_score": 0.5}],
            "model": "rerank-2",
            "usage": {"total_tokens": 3}
        });
        VoyageProvider.transform_rerank_response(&mut body).unwrap();
        assert!(body.get("data").is_none());
        assert_eq!(body["results"][0]["index"], 2);
        assert_eq!(body["results"][1]["relevance_score"], 0.5);
    }

    #[test]
    fn test_voyage_rerank_reply_string_document_becomes_text_object() {
        let mut body = json!({
            "data": [{"index": 0, "relevance_score": 0.9, "document": "alpha"}]
        });
        VoyageProvider.transform_rerank_response(&mut body).unwrap();
        assert_eq!(body["results"][0]["document"], json!({"text": "alpha"}));
    }

    #[test]
    fn test_voyage_rerank_reply_with_results_is_unchanged() {
        let original = json!({
            "id": "r1",
            "results": [{"index": 0, "relevance_score": 0.9}],
            "data": [{"index": 5, "relevance_score": 0.1}]
        });
        let mut body = original.clone();
        VoyageProvider.transform_rerank_response(&mut body).unwrap();
        assert_eq!(body, original);
    }

    #[test]
    fn test_voyage_rerank_request_top_n_becomes_top_k() {
        let mut body = json!({"model": "rerank-2", "query": "q", "documents": ["a"], "top_n": 1});
        VoyageProvider.transform_rerank_request(&mut body).unwrap();
        assert_eq!(
            body,
            json!({"model": "rerank-2", "query": "q", "documents": ["a"], "top_k": 1})
        );
    }

    #[test]
    fn test_voyage_embedding_reply_is_not_reshaped() {
        let original = json!({
            "object": "list",
            "data": [{"object": "embedding", "embedding": [0.1, 0.2], "index": 0}],
            "model": "voyage-3",
            "usage": {"total_tokens": 3}
        });
        let mut body = original.clone();
        VoyageProvider.transform_response(&mut body).unwrap();
        assert_eq!(body, original);
    }

    #[test]
    fn test_voyage_rerank_reply_that_is_not_an_object_is_left_alone() {
        for original in [
            json!([{"index": 0, "relevance_score": 0.9}]),
            json!("text"),
            json!(null),
        ] {
            let mut body = original.clone();
            VoyageProvider
                .transform_rerank_response(&mut body)
                .expect("a reply that is not an object is not an error here");
            assert_eq!(body, original);
        }
    }

    #[test]
    fn test_voyage_rerank_reply_with_null_results_uses_data() {
        let mut body = json!({
            "results": null,
            "data": [{"index": 1, "relevance_score": 0.7}]
        });
        VoyageProvider.transform_rerank_response(&mut body).unwrap();
        assert!(body.get("data").is_none());
        assert_eq!(body["results"], json!([{"index": 1, "relevance_score": 0.7}]));
    }

    #[test]
    fn test_voyage_rerank_reply_with_data_that_is_not_an_array_is_left_alone() {
        for data in [json!(null), json!({"index": 0}), json!("text")] {
            let original = json!({"data": data});
            let mut body = original.clone();
            VoyageProvider.transform_rerank_response(&mut body).unwrap();
            assert_eq!(body, original);
        }
    }

    #[test]
    fn test_voyage_rerank_reply_with_empty_data_becomes_empty_results() {
        let mut body = json!({"data": []});
        VoyageProvider.transform_rerank_response(&mut body).unwrap();
        assert_eq!(body, json!({"results": []}));
    }

    #[test]
    fn test_voyage_rerank_reply_items_that_are_not_objects_do_not_panic() {
        let mut body = json!({"data": [1, null, "x", [], {"index": 0, "relevance_score": 0.1, "document": 5}]});
        VoyageProvider.transform_rerank_response(&mut body).unwrap();
        assert_eq!(body["results"].as_array().map(Vec::len), Some(5));
        assert_eq!(body["results"][4]["document"], 5);
    }

    #[test]
    fn test_voyage_prefix_is_case_sensitive_like_the_other_prefixes() {
        use crate::provider::detect_builtin_provider;

        let named = detect_builtin_provider("voyage/rerank-2").expect("voyage/ names a provider");
        assert_eq!(named.name(), "voyage");
        for model in [
            "Voyage/rerank-2",
            "VOYAGE/rerank-2",
            "Cohere/rerank-v3.5",
            "Azure/gpt-4o",
        ] {
            assert!(
                detect_builtin_provider(model).is_none(),
                "{model} must not name a provider"
            );
        }
    }

    #[test]
    fn test_voyage_prefix_needs_the_slash() {
        use crate::provider::detect_builtin_provider;

        let named = detect_builtin_provider("voyage/rerank-2").expect("voyage/ names a provider");
        assert_eq!(named.name(), "voyage");
        for model in ["voyage-3", "voyage-rerank-2", "voyageai/x", "voyage"] {
            assert!(
                detect_builtin_provider(model).is_none(),
                "{model} has no voyage/ prefix and must not name the Voyage provider"
            );
        }
    }
}
