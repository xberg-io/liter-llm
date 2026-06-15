use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use liter_llm::tenant::{InMemoryKeyResolver, KeyResolver, KeyResolverError, ResolvedKey, TenantContext, TenantId};
use liter_llm::tower::budget::{
    BudgetDimension, BudgetLedger, BudgetVerdict, CostCheckContext, CostRecordContext, DimensionLimits,
    InMemoryBudgetLedger,
};

#[cfg(feature = "tower")]
use liter_llm::tower::types::LlmRequest;

// ── TenantId and TenantContext ────────────────────────────────────────────────

#[test]
fn tenant_id_from_str() {
    let id = TenantId::from("acme-corp");
    assert_eq!(id.as_ref(), "acme-corp");
    assert_eq!(id.to_string(), "acme-corp");
}

#[test]
fn tenant_id_from_string() {
    let id = TenantId::from("acme-corp".to_owned());
    assert_eq!(id.as_ref(), "acme-corp");
}

#[test]
fn tenant_id_eq_and_hash() {
    let a = TenantId::from("acme");
    let b = TenantId::from("acme");
    assert_eq!(a, b);

    let mut map = HashMap::new();
    map.insert(a, 1u32);
    assert_eq!(map.get(&b), Some(&1));
}

#[test]
fn tenant_context_builder() {
    let ctx = TenantContext::new("acme")
        .with_user_id("alice")
        .with_attribute("region", "eu-west-1");

    assert_eq!(ctx.tenant_id.as_ref(), "acme");
    assert_eq!(ctx.user_id.as_deref(), Some("alice"));
    assert_eq!(ctx.attributes.get("region").map(String::as_str), Some("eu-west-1"));
}

// ── LlmRequest tenant_id round-trip ──────────────────────────────────────────

#[cfg(feature = "tower")]
#[test]
fn tenant_id_round_trip() {
    use liter_llm::types::{ChatCompletionRequest, Message, SystemMessage};

    let req = LlmRequest::Chat(ChatCompletionRequest {
        model: "gpt-4".into(),
        messages: vec![Message::System(SystemMessage {
            content: "test".into(),
            name: None,
        })],
        ..Default::default()
    })
    .with_tenant_id("acme-corp");

    assert_eq!(req.tenant_id(), Some(&TenantId::from("acme-corp")));
}

#[cfg(feature = "tower")]
#[test]
fn request_without_tenant_returns_none() {
    use liter_llm::types::{ChatCompletionRequest, Message, SystemMessage};

    let req = LlmRequest::Chat(ChatCompletionRequest {
        model: "gpt-4".into(),
        messages: vec![Message::System(SystemMessage {
            content: "test".into(),
            name: None,
        })],
        ..Default::default()
    });

    assert_eq!(req.tenant_id(), None);
}

// ── InMemoryKeyResolver ───────────────────────────────────────────────────────

fn sample_key(tenant: &str, active: bool) -> ResolvedKey {
    ResolvedKey {
        tenant_id: TenantId::from(tenant),
        allowed_models: vec!["gpt-4".into()],
        monthly_budget: None,
        currency: None,
        metadata: HashMap::new(),
        active,
    }
}

#[tokio::test]
async fn in_memory_resolver_basic() {
    let resolver = InMemoryKeyResolver::new();
    resolver.insert("sk-acme-key", sample_key("acme", true));

    let resolved = resolver.resolve("sk-acme-key").await.expect("should resolve");
    assert_eq!(resolved.tenant_id.as_ref(), "acme");
    assert_eq!(resolved.allowed_models, vec!["gpt-4"]);
    assert!(resolved.active);
}

#[tokio::test]
async fn in_memory_resolver_not_found() {
    let resolver = InMemoryKeyResolver::new();

    let err = resolver.resolve("sk-nonexistent").await.expect_err("should fail");
    assert!(matches!(err, KeyResolverError::NotFound));
}

#[tokio::test]
async fn in_memory_resolver_inactive() {
    let resolver = InMemoryKeyResolver::new();
    resolver.insert("sk-disabled", sample_key("acme", false));

    let err = resolver
        .resolve("sk-disabled")
        .await
        .expect_err("should fail for inactive key");
    assert!(matches!(err, KeyResolverError::Inactive));
}

#[tokio::test]
async fn in_memory_resolver_remove() {
    let resolver = InMemoryKeyResolver::new();
    resolver.insert("sk-temp", sample_key("acme", true));

    assert!(resolver.resolve("sk-temp").await.is_ok());
    let removed = resolver.remove("sk-temp");
    assert!(removed.is_some());
    assert!(resolver.resolve("sk-temp").await.is_err());
}

#[tokio::test]
async fn in_memory_resolver_with_entries() {
    let entries = vec![
        ("sk-a".to_owned(), sample_key("tenant-a", true)),
        ("sk-b".to_owned(), sample_key("tenant-b", true)),
    ];
    let resolver = InMemoryKeyResolver::with_entries(entries);

    let a = resolver.resolve("sk-a").await.expect("should resolve a");
    assert_eq!(a.tenant_id.as_ref(), "tenant-a");

    let b = resolver.resolve("sk-b").await.expect("should resolve b");
    assert_eq!(b.tenant_id.as_ref(), "tenant-b");
}

// ── BudgetLedger reads Tenant dimension from request ─────────────────────────

#[tokio::test]
async fn budget_ledger_reads_tenant_from_request_context() {
    let mut limits = DimensionLimits::default();
    limits.per_tenant.insert("acme".to_owned(), 0.05);

    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    // Simulate: after resolving the request's tenant_id, populate CostRecordContext.
    let tenant_id_str = "acme";

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some(tenant_id_str),
            user_id: None,
            api_key_id: None,
            cost_usd: 0.10,
            tokens_in: 1000,
            tokens_out: 500,
            timestamp: std::time::SystemTime::now(),
        })
        .await;

    // Check should reject: 0.10 > 0.05 limit.
    let verdict = ledger
        .check(&CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some(tenant_id_str),
            user_id: None,
            api_key_id: None,
            timestamp: std::time::SystemTime::now(),
        })
        .await;

    match verdict {
        BudgetVerdict::Reject { dimension, .. } => {
            assert!(
                matches!(&dimension, BudgetDimension::Tenant(t) if t == "acme"),
                "expected Tenant(acme) dimension, got {dimension:?}"
            );
        }
        BudgetVerdict::Allow => panic!("expected Reject, got Allow"),
    }
}

#[tokio::test]
async fn budget_ledger_skips_tenant_check_when_none() {
    let mut limits = DimensionLimits::default();
    limits.per_tenant.insert("acme".to_owned(), 0.01);

    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    // Record spend that would exceed acme's limit — but this context has no tenant.
    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: None,
            api_key_id: None,
            cost_usd: 10.0,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: std::time::SystemTime::now(),
        })
        .await;

    // Check without tenant — should allow (no tenant limit applies to unidentified traffic).
    let verdict = ledger
        .check(&CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: None,
            api_key_id: None,
            timestamp: std::time::SystemTime::now(),
        })
        .await;

    assert!(
        matches!(verdict, BudgetVerdict::Allow),
        "missing tenant_id should not trigger tenant budget check"
    );
}

// ── KeyResolver object-safety ─────────────────────────────────────────────────

#[tokio::test]
async fn key_resolver_is_object_safe() {
    let resolver: Arc<dyn KeyResolver> = Arc::new(InMemoryKeyResolver::new());
    let err = resolver.resolve("anything").await.unwrap_err();
    assert!(matches!(err, KeyResolverError::NotFound));
}
