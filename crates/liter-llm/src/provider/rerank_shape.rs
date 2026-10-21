//! Rerank reply shapes that differ from the canonical `results` array.
//!
//! One table names the shape of each provider, and one function applies it. A provider
//! whose rerank reply is not canonical adds a row here; it adds no parser of its own.

use serde_json::{Value, json};

/// Where a reply keeps its scored documents.
enum ListAt {
    /// Under this key of the top-level object.
    Key(&'static str),
    /// As the top-level array itself.
    TopLevel,
}

/// How one provider words its rerank reply.
struct Shape {
    list: ListAt,
    /// The name of the score field when it is not `relevance_score`.
    score_field: Option<&'static str>,
}

fn shape_of(provider: &str) -> Option<Shape> {
    match provider {
        // Voyage and Fireworks list the hits under `data`.
        "voyage" | "fireworks_ai" => Some(Shape {
            list: ListAt::Key("data"),
            score_field: None,
        }),
        "nvidia_nim" => Some(Shape {
            list: ListAt::Key("rankings"),
            score_field: Some("logit"),
        }),
        // Hugging Face Text Embeddings Inference answers with a bare array.
        "huggingface" => Some(Shape {
            list: ListAt::TopLevel,
            score_field: Some("score"),
        }),
        _ => None,
    }
}

/// Map the rerank reply of `provider` onto the canonical shape.
///
/// A provider with no row in the table, and a reply that already has a `results` value other
/// than `null`, are left alone, so a gateway that normalizes replies still parses. A reply
/// whose list is missing or is not an array is left alone too and fails the parse as before.
/// Items that are not objects, or lack an `index` or a score, stay as they are and fail the
/// parse with an error that names the field.
pub(crate) fn reshape_rerank_reply(provider: &str, body: &mut Value) {
    let Some(shape) = shape_of(provider) else {
        return;
    };
    let mut items = match (&shape.list, &mut *body) {
        (ListAt::TopLevel, Value::Array(_)) => body.take(),
        (ListAt::Key(key), Value::Object(obj)) => {
            if obj.get("results").is_some_and(|results| !results.is_null()) {
                return;
            }
            match obj.remove(*key) {
                Some(list) if list.is_array() => list,
                Some(other) => {
                    obj.insert((*key).to_owned(), other);
                    return;
                }
                None => return,
            }
        }
        _ => return,
    };
    for item in items.as_array_mut().into_iter().flatten() {
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        if let Some(field) = shape.score_field
            && !item.contains_key("relevance_score")
            && let Some(score) = item.remove(field)
        {
            item.insert("relevance_score".to_owned(), score);
        }
        if let Some(doc) = item.get_mut("document")
            && let Some(text) = doc.as_str()
        {
            *doc = json!({ "text": text });
        }
    }
    match body {
        Value::Object(obj) => {
            obj.insert("results".to_owned(), items);
        }
        _ => *body = json!({ "results": items }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nim_rankings_become_results_with_the_logit_as_score() {
        let mut body = json!({"rankings": [{"index": 2, "logit": 0.2}, {"index": 0, "logit": -1.0}]});
        reshape_rerank_reply("nvidia_nim", &mut body);
        assert_eq!(
            body,
            json!({"results": [{"index": 2, "relevance_score": 0.2}, {"index": 0, "relevance_score": -1.0}]})
        );
    }

    #[test]
    fn tei_array_becomes_results_with_the_score_renamed() {
        let mut body = json!([{"index": 2, "score": 0.9}, {"index": 0, "score": 0.5}]);
        reshape_rerank_reply("huggingface", &mut body);
        assert_eq!(
            body,
            json!({"results": [{"index": 2, "relevance_score": 0.9}, {"index": 0, "relevance_score": 0.5}]})
        );
    }

    #[test]
    fn fireworks_data_becomes_results() {
        let mut body = json!({"data": [{"index": 2, "relevance_score": 0.9}], "model": "m"});
        reshape_rerank_reply("fireworks_ai", &mut body);
        assert_eq!(
            body,
            json!({"model": "m", "results": [{"index": 2, "relevance_score": 0.9}]})
        );
    }

    #[test]
    fn a_relevance_score_wins_over_the_provider_score_field() {
        let mut body = json!({"rankings": [{"index": 0, "logit": 5.0, "relevance_score": 0.3}]});
        reshape_rerank_reply("nvidia_nim", &mut body);
        assert_eq!(body["results"][0]["relevance_score"], 0.3);
        assert_eq!(body["results"][0]["logit"], 5.0);
    }

    #[test]
    fn a_provider_with_no_row_and_a_canonical_reply_are_left_alone() {
        let cases = [
            ("cohere", json!({"rankings": [{"index": 0, "logit": 1.0}]})),
            (
                "nvidia_nim",
                json!({"results": [{"index": 0, "relevance_score": 0.1}], "rankings": []}),
            ),
            ("nvidia_nim", json!([{"index": 0, "logit": 1.0}])),
            ("huggingface", json!({"results": []})),
            ("huggingface", json!({"data": [{"index": 0, "score": 1.0}]})),
            ("nvidia_nim", json!({"rankings": "text"})),
            ("huggingface", json!("text")),
        ];
        for (provider, original) in cases {
            let mut body = original.clone();
            reshape_rerank_reply(provider, &mut body);
            assert_eq!(body, original, "{provider}");
        }
    }

    #[test]
    fn items_that_are_not_objects_do_not_panic() {
        let mut body = json!([1, null, "x", []]);
        reshape_rerank_reply("huggingface", &mut body);
        assert_eq!(body, json!({"results": [1, null, "x", []]}));
    }
}
