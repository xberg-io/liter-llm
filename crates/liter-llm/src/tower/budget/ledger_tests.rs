//! Tests for [`InMemoryBudgetLedger`], [`BudgetLedger`] snapshots, and [`should_hedge`].

use std::sync::Arc;
use std::time::Duration;

use super::*;

#[tokio::test]
async fn budget_ledger_records_per_key_and_per_user() {
    let limits = DimensionLimits::default();
    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    let ctx1 = CostRecordContext {
        model: "gpt-4",
        provider: "openai",
        tenant_id: Some("acme"),
        user_id: Some("alice"),
        api_key_id: Some("key-1"),
        cost_usd: 0.10,
        tokens_in: 1000,
        tokens_out: 500,
        timestamp: SystemTime::now(),
    };
    ledger.record(&ctx1).await;

    let ctx2 = CostRecordContext {
        model: "gpt-4",
        provider: "openai",
        tenant_id: Some("acme"),
        user_id: Some("bob"),
        api_key_id: Some("key-2"),
        cost_usd: 0.20,
        tokens_in: 2000,
        tokens_out: 1000,
        timestamp: SystemTime::now(),
    };
    ledger.record(&ctx2).await;

    let snap = ledger.snapshot();
    assert!(
        (snap.global_spend_usd - 0.30).abs() < 1e-9,
        "global: {}",
        snap.global_spend_usd
    );
    assert!((snap.per_model["gpt-4"] - 0.30).abs() < 1e-9);
    assert!((snap.per_tenant["acme"] - 0.30).abs() < 1e-9);
    assert!((snap.per_user["alice"] - 0.10).abs() < 1e-9);
    assert!((snap.per_user["bob"] - 0.20).abs() < 1e-9);
    assert!((snap.per_api_key["key-1"] - 0.10).abs() < 1e-9);
    assert!((snap.per_api_key["key-2"] - 0.20).abs() < 1e-9);
}

/// `per_user` is keyed by the request's free-text, attacker-controlled
/// `user` field, so without a cap a caller grows the map without bound
/// simply by varying that field per request. This pins the cap: recording
/// spend for far more distinct users than the configured cap must never
/// let `per_user` grow past it.
#[tokio::test]
async fn budget_ledger_per_user_map_is_bounded_by_max_principal_entries() {
    const CAP: usize = 8;
    let ledger = InMemoryBudgetLedger::new(DimensionLimits::default(), Duration::from_secs(3600))
        .with_max_principal_entries(CAP);

    for i in 0..CAP * 10 {
        let user = format!("user-{i}");
        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some(&user),
                api_key_id: None,
                cost_usd: 0.01,
                tokens_in: 10,
                tokens_out: 5,
                timestamp: SystemTime::now(),
            })
            .await;
    }

    let len = ledger.per_user.len();
    assert!(
        len <= CAP,
        "per_user must stay within its cap; held {len} entries with a cap of {CAP}"
    );
}

/// Same regression as above, but for `per_api_key` — the other
/// caller-supplied, unbounded-cardinality dimension.
#[tokio::test]
async fn budget_ledger_per_api_key_map_is_bounded_by_max_principal_entries() {
    const CAP: usize = 8;
    let ledger = InMemoryBudgetLedger::new(DimensionLimits::default(), Duration::from_secs(3600))
        .with_max_principal_entries(CAP);

    for i in 0..CAP * 10 {
        let key = format!("key-{i}");
        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: None,
                api_key_id: Some(&key),
                cost_usd: 0.01,
                tokens_in: 10,
                tokens_out: 5,
                timestamp: SystemTime::now(),
            })
            .await;
    }

    let len = ledger.per_api_key.len();
    assert!(
        len <= CAP,
        "per_api_key must stay within its cap; held {len} entries with a cap of {CAP}"
    );
}

/// Zero-spend entries carry no enforcement value, so reaching the cap
/// must first reclaim them before declining to track a new principal —
/// the same "reclaim before giving up" shape as
/// `ClassifierVerdictCache::put_cached`, but keyed on recorded spend
/// rather than TTL expiry.
#[tokio::test]
async fn budget_ledger_cap_reclaims_zero_spend_entries_before_declining_new_principals() {
    const CAP: usize = 4;
    let ledger = InMemoryBudgetLedger::new(DimensionLimits::default(), Duration::from_secs(3600))
        .with_max_principal_entries(CAP);

    for i in 0..CAP {
        let user = format!("zero-spend-{i}");
        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some(&user),
                api_key_id: None,
                cost_usd: 0.0,
                tokens_in: 0,
                tokens_out: 0,
                timestamp: SystemTime::now(),
            })
            .await;
    }
    assert_eq!(ledger.per_user.len(), CAP, "setup should fill the ledger to its cap");

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("real-spender"),
            api_key_id: None,
            cost_usd: 5.0,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    assert_eq!(
        ledger.per_user.len(),
        1,
        "the zero-spend entries must be reclaimed, leaving only the new spender"
    );
    assert!((ledger.snapshot().per_user["real-spender"] - 5.0).abs() < 1e-9);
}

/// The core safety property this cap must uphold: filling the ledger to
/// capacity with unrelated new principals must never reset (via eviction)
/// a principal's already-recorded, non-zero spend. If it did, the
/// bounded-memory fix would itself become a budget-bypass — a caller
/// could spend heavily, get displaced by cardinality pressure from other
/// keys, and come back with a silently fresh budget.
#[tokio::test]
async fn budget_ledger_cap_never_resets_an_existing_principals_spend() {
    const CAP: usize = 4;
    let ledger = InMemoryBudgetLedger::new(DimensionLimits::default(), Duration::from_secs(3600))
        .with_max_principal_entries(CAP);

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 42.0,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;
    assert!((ledger.snapshot().per_user["alice"] - 42.0).abs() < 1e-9);

    // Flood with far more distinct, non-zero-spend principals than the cap allows.
    for i in 0..CAP * 10 {
        let user = format!("flood-{i}");
        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some(&user),
                api_key_id: None,
                cost_usd: 1.0,
                tokens_in: 10,
                tokens_out: 5,
                timestamp: SystemTime::now(),
            })
            .await;
    }

    let spend_after_flood = ledger.snapshot().per_user.get("alice").copied();
    assert!(
        matches!(spend_after_flood, Some(v) if (v - 42.0).abs() < 1e-9),
        "alice's already-recorded spend must survive cap pressure from other principals, got {spend_after_flood:?}"
    );

    // An already-tracked principal must keep accumulating spend even while
    // the ledger sits at capacity -- the cap must gate only new principals.
    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 8.0,
            tokens_in: 10,
            tokens_out: 5,
            timestamp: SystemTime::now(),
        })
        .await;
    assert!(
        (ledger.snapshot().per_user["alice"] - 50.0).abs() < 1e-9,
        "an already-tracked principal must keep accumulating spend once the ledger is at capacity"
    );
}

/// Regression for the reset-on-reload bug: `update_limits` must swap only
/// the caps, not the accumulated spend. Before the fix, the only way to
/// change limits was to rebuild the whole ledger, which zeroed every
/// `DashMap` entry along with it — this test fails against that
/// rebuild-the-ledger behaviour because the post-update spend would read
/// back as 0.0 instead of the pre-update amount.
#[tokio::test]
async fn update_limits_preserves_accumulated_spend() {
    let mut limits = DimensionLimits::default();
    limits.per_user.insert("alice".to_owned(), 100.0);
    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 7.50,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;
    assert!((ledger.snapshot().per_user["alice"] - 7.50).abs() < 1e-9);

    let mut new_limits = DimensionLimits::default();
    new_limits.per_user.insert("alice".to_owned(), 200.0);
    ledger.update_limits(new_limits);

    let snap = ledger.snapshot();
    assert!(
        (snap.per_user["alice"] - 7.50).abs() < 1e-9,
        "spend must survive update_limits, got {}",
        snap.per_user["alice"]
    );
    assert_eq!(snap.limits_per_user.get("alice"), Some(&200.0));

    let verdict = ledger
        .check(&CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            timestamp: SystemTime::now(),
        })
        .await;
    assert!(
        matches!(verdict, BudgetVerdict::Allow),
        "spend $7.50 is well under the new $200 limit, expected Allow, got {verdict:?}"
    );
}

/// Lowering a limit below already-accumulated spend must reject the very
/// next request rather than silently forgiving the existing spend — the
/// opposite failure mode (forgiving spend) is the reset-on-reload bug in
/// disguise.
#[tokio::test]
async fn update_limits_lowering_below_spend_rejects_immediately() {
    let mut limits = DimensionLimits::default();
    limits.per_user.insert("alice".to_owned(), 100.0);
    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 5.0,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    let mut lowered = DimensionLimits::default();
    lowered.per_user.insert("alice".to_owned(), 1.0);
    ledger.update_limits(lowered);

    let verdict = ledger
        .check(&CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            timestamp: SystemTime::now(),
        })
        .await;
    match verdict {
        BudgetVerdict::Reject { dimension, .. } => {
            assert!(matches!(dimension, BudgetDimension::User(ref u) if u == "alice"));
        }
        BudgetVerdict::Allow => panic!("lowering the limit below existing spend must reject, got Allow"),
    }
}

/// A tenant dropped from the limits map on reload must keep its spend
/// history: enforcement lifts (no configured cap means no rejection) but
/// the `DashMap` window entry is retained, so re-adding the same tenant
/// later does not silently reset it to zero.
#[tokio::test]
async fn update_limits_retains_spend_for_removed_tenant() {
    let mut limits = DimensionLimits::default();
    limits.per_tenant.insert("acme".to_owned(), 10.0);
    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some("acme"),
            user_id: None,
            api_key_id: None,
            cost_usd: 3.0,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    // ~keep "acme" is absent from the new limits map entirely, simulating removal from config.
    ledger.update_limits(DimensionLimits::default());

    let verdict = ledger
        .check(&CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some("acme"),
            user_id: None,
            api_key_id: None,
            timestamp: SystemTime::now(),
        })
        .await;
    assert!(
        matches!(verdict, BudgetVerdict::Allow),
        "no configured limit means no enforcement, expected Allow, got {verdict:?}"
    );

    let snap = ledger.snapshot();
    assert!(
        (snap.per_tenant["acme"] - 3.0).abs() < 1e-9,
        "spend history for a removed tenant must be retained, got {:?}",
        snap.per_tenant.get("acme")
    );
}

#[tokio::test]
async fn budget_ledger_rejects_when_user_limit_exceeded() {
    let mut limits = DimensionLimits::default();
    limits.per_user.insert("alice".to_owned(), 0.05);

    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 0.10,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    let verdict = ledger
        .check(&CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            timestamp: SystemTime::now(),
        })
        .await;

    match verdict {
        BudgetVerdict::Reject { dimension, .. } => {
            assert!(
                matches!(dimension, BudgetDimension::User(ref u) if u == "alice"),
                "expected User(alice) dimension, got {dimension:?}"
            );
        }
        BudgetVerdict::Allow => panic!("expected Reject, got Allow"),
    }
}

#[tokio::test]
async fn budget_ledger_resets_at_window_boundary() {
    let limits = DimensionLimits {
        global: Some(100.0),
        ..Default::default()
    };
    let window = Duration::from_secs(1);
    let ledger = InMemoryBudgetLedger::new(limits, window);

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: None,
            api_key_id: None,
            cost_usd: 50.0,
            tokens_in: 1_000_000,
            tokens_out: 0,
            timestamp: SystemTime::now(),
        })
        .await;

    assert!(ledger.snapshot().global_spend_usd > 0.0);

    let future = SystemTime::now() + Duration::from_secs(2);

    let spend_after_window = ledger.global.spend_usd(future);
    assert_eq!(spend_after_window, 0.0, "spend should reset to 0 after window boundary");
}

#[tokio::test]
async fn budget_snapshot_csv_export_round_trips() {
    let ledger = InMemoryBudgetLedger::new(DimensionLimits::default(), Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some("tenant-x"),
            user_id: Some("user-y"),
            api_key_id: Some("key-z"),
            cost_usd: 1.23,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    let mut csv_bytes: Vec<u8> = Vec::new();
    ledger.export_csv(&mut csv_bytes).expect("CSV export must not fail");
    let csv = String::from_utf8(csv_bytes).expect("CSV must be valid UTF-8");

    assert!(csv.starts_with("dimension,spend_usd\n"), "missing header: {csv}");

    let mut found_global = false;
    let mut found_model = false;
    let mut found_tenant = false;
    let mut found_user = false;
    let mut found_key = false;

    for line in csv.lines().skip(1) {
        let parts: Vec<&str> = line.splitn(2, ',').collect();
        assert_eq!(parts.len(), 2, "malformed CSV line: {line}");
        let dimension = parts[0];
        let spend: f64 = parts[1].parse().expect("spend must be a float");

        match dimension {
            "global" => {
                assert!((spend - 1.23).abs() < 1e-6, "global spend mismatch: {spend}");
                found_global = true;
            }
            "model:gpt-4" => {
                assert!((spend - 1.23).abs() < 1e-6);
                found_model = true;
            }
            "tenant:tenant-x" => {
                assert!((spend - 1.23).abs() < 1e-6);
                found_tenant = true;
            }
            "user:user-y" => {
                assert!((spend - 1.23).abs() < 1e-6);
                found_user = true;
            }
            "api_key:key-z" => {
                assert!((spend - 1.23).abs() < 1e-6);
                found_key = true;
            }
            _ => {}
        }
    }

    assert!(found_global, "global row missing from CSV");
    assert!(found_model, "model row missing from CSV");
    assert!(found_tenant, "tenant row missing from CSV");
    assert!(found_user, "user row missing from CSV");
    assert!(found_key, "api_key row missing from CSV");
}

/// Spawn 100 threads each calling `add($0.10)` exactly at the window
/// boundary and assert the total is $10.00, not less.
///
/// The CAS in `spend_usd` guarantees exactly one thread resets the window;
/// the other 99 threads see the already-zeroed counter but still add their
/// $0.10 contribution via `fetch_add`.  Without the CAS fix, both threads
/// that race on the boundary would zero `spend_mc` independently, causing
/// each other's prior `add` to be dropped.
#[test]
fn window_rollover_under_concurrent_threads_does_not_undercount() {
    use std::sync::Barrier;
    use std::thread;

    let entry = Arc::new(WindowEntry::new(Duration::from_secs(1)));

    let future_now = SystemTime::now() + Duration::from_secs(2);

    let barrier = Arc::new(Barrier::new(100));
    let mut handles = Vec::with_capacity(100);

    for _ in 0..100 {
        let entry_clone = Arc::clone(&entry);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            entry_clone.add(0.10, future_now);
        }));
    }

    for h in handles {
        h.join().expect("thread must not panic");
    }

    let total = microcents_to_usd(entry.spend_mc.load(Ordering::Acquire));
    assert!(
        (total - 10.0_f64).abs() < 1e-4,
        "expected $10.00 total, got ${total:.6} — window rollover race caused under-counting"
    );
}

/// Bug 6 fix: 200 parallel `add($0.10)` calls at a rollover boundary must
/// total exactly $20.00 — no contribution lost due to TOCTOU.
#[test]
fn budget_window_rollover_no_torn_read() {
    use std::sync::Barrier;
    use std::thread;

    let entry = Arc::new(WindowEntry::new(Duration::from_secs(1)));
    let future_now = SystemTime::now() + Duration::from_secs(2);

    const WRITERS: usize = 200;
    let barrier = Arc::new(Barrier::new(WRITERS));
    let mut handles = Vec::with_capacity(WRITERS);
    for _ in 0..WRITERS {
        let e = Arc::clone(&entry);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            e.add(0.10, future_now);
        }));
    }
    for h in handles {
        h.join().expect("writer must not panic");
    }
    let total = microcents_to_usd(entry.spend_mc.load(Ordering::Acquire));
    assert!(
        (total - 20.0_f64).abs() < 1e-4,
        "expected $20.00 total after 200 concurrent adds at rollover; got ${total:.6}"
    );
}

/// $10 user budget, $9.50 spend, estimated_cost=$0.50, safety_margin=0.10
/// → effective limit = $10 × 0.90 = $9.00.
/// $9.50 + 2×$0.50 = $10.50 ≥ $9.00 → hedging must be suppressed.
#[tokio::test]
async fn should_hedge_respects_user_budget() {
    let mut limits = DimensionLimits::default();
    limits.per_user.insert("alice".to_owned(), 10.0);

    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 9.50,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    let ctx = CostCheckContext {
        model: "gpt-4",
        provider: "openai",
        tenant_id: None,
        user_id: Some("alice"),
        api_key_id: None,
        timestamp: SystemTime::now(),
    };

    let result = should_hedge(&ledger, &ctx, 0.50, 0.10);
    assert!(
        !result,
        "hedging should be suppressed when user spend + 2×cost would exceed 90% of budget"
    );
}

/// Same $10 user budget but only $1.00 spend.
/// $1.00 + 2×$0.50 = $2.00 < $9.00 → hedging must be allowed.
#[tokio::test]
async fn should_hedge_allows_when_far_below_budget() {
    let mut limits = DimensionLimits::default();
    limits.per_user.insert("alice".to_owned(), 10.0);

    let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

    ledger
        .record(&CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            cost_usd: 1.00,
            tokens_in: 100,
            tokens_out: 50,
            timestamp: SystemTime::now(),
        })
        .await;

    let ctx = CostCheckContext {
        model: "gpt-4",
        provider: "openai",
        tenant_id: None,
        user_id: Some("alice"),
        api_key_id: None,
        timestamp: SystemTime::now(),
    };

    let result = should_hedge(&ledger, &ctx, 0.50, 0.10);
    assert!(
        result,
        "hedging should be allowed when user spend + 2×cost is well below 90% of budget"
    );
}
