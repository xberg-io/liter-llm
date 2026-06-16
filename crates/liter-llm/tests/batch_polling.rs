#![cfg(all(test, any(feature = "native-http", feature = "wasm-http")))]

use std::sync::{Arc, Mutex};

use liter_llm::client::{BatchRetriever, BatchWaitError, WaitForBatchConfig, wait_for_batch_impl};
use liter_llm::error::Result;
use liter_llm::types::batch::{BatchObject, BatchStatus};

struct StubRetriever {
    statuses: Arc<Mutex<Vec<BatchStatus>>>,
    call_count: Arc<Mutex<usize>>,
}

impl StubRetriever {
    fn new(statuses: Vec<BatchStatus>) -> Self {
        Self {
            // `pop` returns the last element first — declared order is reverse-chronological.
            statuses: Arc::new(Mutex::new(statuses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    fn infinite(status: BatchStatus) -> Self {
        Self {
            statuses: Arc::new(Mutex::new(vec![status; 100])),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    fn calls(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait::async_trait]
impl BatchRetriever for StubRetriever {
    async fn fetch_batch_for_polling(&self, _batch_id: &str) -> Result<BatchObject> {
        *self.call_count.lock().unwrap() += 1;
        let mut statuses = self.statuses.lock().unwrap();
        let status = statuses.pop().unwrap_or(BatchStatus::Completed);

        Ok(BatchObject {
            id: "b-test".to_string(),
            object: "batch".to_string(),
            endpoint: "/v1/chat/completions".to_string(),
            input_file_id: "file-in".to_string(),
            completion_window: "24h".to_string(),
            status,
            output_file_id: None,
            error_file_id: None,
            created_at: 0,
            completed_at: None,
            failed_at: None,
            expired_at: None,
            request_counts: None,
            metadata: None,
        })
    }
}

#[tokio::test(start_paused = true)]
async fn polls_until_completed() {
    let stub = StubRetriever::new(vec![
        BatchStatus::Completed,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::Validating,
    ]);
    let result = wait_for_batch_impl(&stub, "b-1", WaitForBatchConfig::default()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, BatchStatus::Completed);
    assert_eq!(stub.calls(), 4);
}

#[tokio::test(start_paused = true)]
async fn terminal_failure_returns_failed_error() {
    let stub = StubRetriever::new(vec![BatchStatus::Failed]);
    let result = wait_for_batch_impl(&stub, "b-1", WaitForBatchConfig::default()).await;
    assert!(matches!(
        result,
        Err(BatchWaitError::Failed {
            status: BatchStatus::Failed
        })
    ));
}

#[tokio::test(start_paused = true)]
async fn expired_status_returns_failed_error() {
    let stub = StubRetriever::new(vec![BatchStatus::Expired]);
    let result = wait_for_batch_impl(&stub, "b-1", WaitForBatchConfig::default()).await;
    assert!(matches!(
        result,
        Err(BatchWaitError::Failed {
            status: BatchStatus::Expired
        })
    ));
}

#[tokio::test(start_paused = true)]
async fn cancelled_status_returns_failed_error() {
    let stub = StubRetriever::new(vec![BatchStatus::Cancelled]);
    let result = wait_for_batch_impl(&stub, "b-1", WaitForBatchConfig::default()).await;
    assert!(matches!(
        result,
        Err(BatchWaitError::Failed {
            status: BatchStatus::Cancelled
        })
    ));
}

#[tokio::test(start_paused = true)]
async fn timeout_returns_timeout_error() {
    let stub = StubRetriever::infinite(BatchStatus::InProgress);
    let config = WaitForBatchConfig {
        timeout_secs: Some(10.0),
        ..Default::default()
    };
    let result = wait_for_batch_impl(&stub, "b-1", config).await;
    assert!(matches!(result, Err(BatchWaitError::Timeout { .. })));
}

#[tokio::test(start_paused = true)]
async fn respects_backoff_curve() {
    let stub = StubRetriever::new(vec![
        BatchStatus::Completed,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::Validating,
    ]);
    let config = WaitForBatchConfig {
        initial_interval_secs: 1.0,
        max_interval_secs: 10.0,
        backoff_multiplier: 2.0,
        timeout_secs: None,
    };

    let start = tokio::time::Instant::now();
    let result = wait_for_batch_impl(&stub, "b-1", config).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(stub.calls(), 5);

    let total_sleep = 1 + 2 + 4 + 8;
    assert_eq!(elapsed.as_secs(), total_sleep);
}

#[tokio::test(start_paused = true)]
async fn respects_max_interval() {
    let stub = StubRetriever::new(vec![
        BatchStatus::Completed,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::Validating,
    ]);
    let config = WaitForBatchConfig {
        initial_interval_secs: 1.0,
        max_interval_secs: 5.0,
        backoff_multiplier: 2.0,
        timeout_secs: None,
    };

    let start = tokio::time::Instant::now();
    let result = wait_for_batch_impl(&stub, "b-1", config).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(stub.calls(), 6);

    let total_sleep = 1 + 2 + 4 + 5 + 5;
    assert_eq!(elapsed.as_secs(), total_sleep);
}

#[tokio::test(start_paused = true)]
async fn timeout_after_multiple_polls() {
    let stub = StubRetriever::new(vec![
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
        BatchStatus::InProgress,
    ]);
    let config = WaitForBatchConfig {
        initial_interval_secs: 3.0,
        max_interval_secs: 10.0,
        backoff_multiplier: 1.5,
        timeout_secs: Some(5.0),
    };

    let result = wait_for_batch_impl(&stub, "b-1", config).await;
    assert!(matches!(result, Err(BatchWaitError::Timeout { .. })));
}

#[tokio::test(start_paused = true)]
async fn finalized_statuses_immediately_return() {
    // Vec is reverse-chronological — pop returns the last element first.
    let stub = StubRetriever::new(vec![BatchStatus::Completed, BatchStatus::Finalizing]);
    let result = wait_for_batch_impl(&stub, "b-1", WaitForBatchConfig::default()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, BatchStatus::Completed);
    assert_eq!(stub.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn cancelling_then_cancelled() {
    let stub = StubRetriever::new(vec![
        BatchStatus::Cancelled,
        BatchStatus::Cancelling,
        BatchStatus::InProgress,
    ]);
    let result = wait_for_batch_impl(&stub, "b-1", WaitForBatchConfig::default()).await;
    assert!(matches!(
        result,
        Err(BatchWaitError::Failed {
            status: BatchStatus::Cancelled
        })
    ));
}
