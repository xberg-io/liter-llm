//! [`Service`] implementation for [`BudgetLedgerService`].

use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;

use tower::Service;

use super::{
    BudgetDimension, BudgetLedger, BudgetLedgerService, BudgetVerdict, CostCheckContext, CostRecordContext,
    Enforcement, provider_of, user_id_of,
};
use crate::client::BoxFuture;
use crate::cost;
use crate::error::{LiterLlmError, Result};
use crate::tower::cost::observe_stream_usage;
use crate::tower::types::{LlmRequest, LlmResponse};
use crate::types::Usage;

/// Owned per-request attribution captured before the request is handed to the
/// inner service, so it outlives the borrowed [`LlmRequest`].
struct LedgerCallMeta {
    model: String,
    provider: String,
    tenant_id: Option<String>,
    user_id: Option<String>,
}

impl LedgerCallMeta {
    fn from_request(req: &LlmRequest) -> Self {
        let model = req.model().unwrap_or("unknown").to_owned();
        let provider = provider_of(&model).to_owned();
        Self {
            tenant_id: req.tenant_id().map(|t| t.as_ref().to_owned()),
            user_id: user_id_of(req).map(str::to_owned),
            model,
            provider,
        }
    }

    fn check_ctx(&self) -> CostCheckContext<'_> {
        CostCheckContext {
            model: &self.model,
            provider: &self.provider,
            tenant_id: self.tenant_id.as_deref(),
            user_id: self.user_id.as_deref(),
            api_key_id: None,
            timestamp: SystemTime::now(),
        }
    }

    fn record_ctx(&self, usd: f64, usage: &Usage) -> CostRecordContext<'_> {
        CostRecordContext {
            model: &self.model,
            provider: &self.provider,
            tenant_id: self.tenant_id.as_deref(),
            user_id: self.user_id.as_deref(),
            api_key_id: None,
            cost_usd: usd,
            tokens_in: usage.prompt_tokens,
            tokens_out: usage.completion_tokens,
            timestamp: SystemTime::now(),
        }
    }

    /// Cost of `usage` for this request's model, or `None` when the model has
    /// no pricing data.
    fn cost_of(&self, usage: &Usage) -> Option<f64> {
        cost::completion_cost(&self.model, usage.prompt_tokens, usage.completion_tokens)
    }
}

/// Map a ledger rejection onto the error returned to the caller.
fn budget_exceeded(reason: String, dimension: &BudgetDimension) -> LiterLlmError {
    let model_field = match dimension {
        BudgetDimension::Model(m) => Some(m.clone()),
        _ => None,
    };
    LiterLlmError::BudgetExceeded {
        message: reason,
        model: model_field,
    }
}

/// Wrap a streamed response so its usage is recorded once the stream completes.
fn record_stream_on_completion<L: BudgetLedger>(
    ledger: Arc<L>,
    meta: LedgerCallMeta,
    resp: LlmResponse,
) -> LlmResponse {
    let LlmResponse::ChatStream(stream) = resp else {
        return resp;
    };
    let wrapped = observe_stream_usage(stream, move |usage| {
        let Some(usage) = usage else { return };
        let Some(usd) = meta.cost_of(&usage) else {
            return;
        };
        // ~keep BudgetLedger::record is async but this callback runs synchronously inside
        // ~keep poll_next; spawn so recording never blocks the caller draining the stream.
        // ~keep Guard against "no current runtime" if the stream is drained outside Tokio (matches
        // ~keep the tokio::runtime::Handle::try_current() convention used by hooks.rs's CancellationGuard).
        let record = async move {
            let ctx = meta.record_ctx(usd, &usage);
            ledger.record(&ctx).await;
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(record);
        } else {
            tracing::warn!(
                "budget ledger: no Tokio runtime available to record streamed usage; spend was not recorded"
            );
        }
    });
    LlmResponse::ChatStream(wrapped)
}

impl<L, S> Service<LlmRequest> for BudgetLedgerService<L, S>
where
    L: BudgetLedger,
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let meta = LedgerCallMeta::from_request(&req);
        let ledger = Arc::clone(&self.ledger);
        let enforcement = self.enforcement;

        // ~keep The pre-flight check is async (ledger-backed), so it must run inside the returned future,
        // ~keep before `inner.call(req)`. Consume the polled-ready instance and leave a fresh standby clone,
        // ~keep matching the Tower contract other layers in this crate follow (e.g. CacheService, HedgeService).
        let standby = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, standby);

        Box::pin(async move {
            if enforcement == Enforcement::Hard
                && let BudgetVerdict::Reject { reason, dimension } = ledger.check(&meta.check_ctx()).await
            {
                return Err(budget_exceeded(reason, &dimension));
            }

            let resp = inner.call(req).await?;

            if matches!(resp, LlmResponse::ChatStream(_)) {
                return Ok(record_stream_on_completion(ledger, meta, resp));
            }
            if let Some(usage) = resp.usage()
                && let Some(usd) = meta.cost_of(usage)
            {
                let ctx = meta.record_ctx(usd, usage);
                ledger.record(&ctx).await;
            }
            Ok(resp)
        })
    }
}
