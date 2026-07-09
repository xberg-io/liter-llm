//! OTel metric helpers for credential pool instrumentation.
//!
//! # Instruments
//!
//! - `gen_ai.credential.rotation` (counter) — a credential was rotated out.
//! - `gen_ai.credential.exhausted` (counter) — a credential was marked exhausted.
//! - `gen_ai.credential.pool.active` (up-down counter) — current active count.
//! - `gen_ai.credential.recovery` (counter) — a credential recovered.
//!
//! All instruments carry a `provider` attribute.
//!
//! The helpers are no-ops when the `otel` feature is disabled or when
//! [`liter_llm::tower::metrics::init_meter`] has not yet been called.

/// Record a credential rotation event (one credential replaced by another).
#[inline]
pub fn record_credential_rotation(provider: &str) {
    #[cfg(feature = "otel")]
    {
        if let Some(meter) = liter_llm::tower::metrics::global_meter() {
            meter
                .u64_counter("gen_ai.credential.rotation")
                .with_description("Number of credential rotation events")
                .build()
                .add(1, &[opentelemetry::KeyValue::new("provider", provider.to_owned())]);
        }
    }
    let _ = provider;
}

/// Record a credential exhaustion event.
#[inline]
pub fn record_credential_exhausted(provider: &str) {
    #[cfg(feature = "otel")]
    {
        if let Some(meter) = liter_llm::tower::metrics::global_meter() {
            meter
                .u64_counter("gen_ai.credential.exhausted")
                .with_description("Number of credential exhaustion events")
                .build()
                .add(1, &[opentelemetry::KeyValue::new("provider", provider.to_owned())]);
        }
    }
    let _ = provider;
}

/// Update the active credential gauge for a provider.
///
/// Uses an `UpDownCounter` so the value can decrease as well as increase.
#[inline]
pub fn record_credential_pool_active(provider: &str, active_count: usize) {
    #[cfg(feature = "otel")]
    {
        if let Some(meter) = liter_llm::tower::metrics::global_meter() {
            meter
                .i64_up_down_counter("gen_ai.credential.pool.active")
                .with_description("Current number of active credentials in the pool")
                .build()
                .add(
                    active_count as i64,
                    &[opentelemetry::KeyValue::new("provider", provider.to_owned())],
                );
        }
    }
    let _ = (provider, active_count);
}

/// Record a credential recovery event (exhausted → active).
#[inline]
pub fn record_credential_recovery(provider: &str) {
    #[cfg(feature = "otel")]
    {
        if let Some(meter) = liter_llm::tower::metrics::global_meter() {
            meter
                .u64_counter("gen_ai.credential.recovery")
                .with_description("Number of credential reactivation events")
                .build()
                .add(1, &[opentelemetry::KeyValue::new("provider", provider.to_owned())]);
        }
    }
    let _ = provider;
}
