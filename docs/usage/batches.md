---
description: "Batch API and polling helper for asynchronous request processing."
---

# Batches

The batch API lets you submit large groups of requests for asynchronous
processing. Submit a JSONL file, then poll for completion. A polling helper
simplifies waiting for results.

## Quick example

Submit a batch job and wait for completion:

```rust
use liter_llm::client::{
    DefaultClient, ClientConfig, WaitForBatchConfig,
    CreateBatchRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = DefaultClient::new(
        ClientConfig::new("sk-..."),
        None
    )?;

    // Upload input file (JSONL format)
    let file_req = liter_llm::types::files::CreateFileRequest {
        file: my_file_bytes,
        purpose: "batch".to_string(),
    };
    let uploaded = client.create_file(file_req).await?;

    // Create batch
    let batch_req = CreateBatchRequest {
        input_file_id: uploaded.id,
        endpoint: "/v1/chat/completions".to_string(),
        completion_window: "24h".to_string(),
        metadata: None,
    };
    let batch = client.create_batch(batch_req).await?;

    // Wait for completion
    let completed = client.wait_for_batch(
        &batch.id,
        WaitForBatchConfig::default()
    ).await?;

    println!("Batch status: {:?}", completed.status);
    println!("Output file: {:?}", completed.output_file_id);

    Ok(())
}
```

## `wait_for_batch`

Poll a batch until it reaches a terminal status:

```rust
pub async fn wait_for_batch(
    &self,
    batch_id: &str,
    config: WaitForBatchConfig,
) -> Result<BatchObject, BatchWaitError>
```

### Configuration

`WaitForBatchConfig` controls polling behaviour:

| Field | Default | Purpose |
|-------|---------|---------|
| `initial_interval` | 5 seconds | First poll delay |
| `max_interval` | 60 seconds | Maximum delay between polls (backoff plateau) |
| `backoff_multiplier` | 1.5 | Exponential backoff factor |
| `timeout` | None | Optional total timeout (no limit if `None`) |

Example with custom delays:

```rust
use std::time::Duration;
use liter_llm::client::WaitForBatchConfig;

let config = WaitForBatchConfig {
    initial_interval: Duration::from_secs(1),
    max_interval: Duration::from_secs(30),
    backoff_multiplier: 2.0,
    timeout: Some(Duration::from_secs(300)), // 5 minutes total
};

let batch = client.wait_for_batch(&batch_id, config).await?;
```

### Terminal statuses

Polling stops when the batch reaches a terminal state:

- `Completed` — all requests succeeded
- `Failed` — validation or processing error (check error file)
- `Expired` — batch window (e.g., "24h") elapsed before completion
- `Cancelled` — batch was manually cancelled

Non-terminal states (polling continues):

- `Validating` — input file validation in progress
- `InProgress` — requests are being processed
- `Finalizing` — computing output file

### Error handling

`BatchWaitError` enumerates poll failures:

```rust
pub enum BatchWaitError {
    /// Batch reached a terminal failure state.
    Failed(BatchStatus),

    /// Polling timed out before reaching terminal status.
    Timeout(Duration),

    /// Underlying client error.
    Client(LiterLlmError),
}
```

Example:

```rust
match client.wait_for_batch(&batch_id, config).await {
    Ok(batch) => println!("Completed: {:?}", batch.status),
    Err(BatchWaitError::Failed(status)) => {
        println!("Batch failed: {:?}", status);
        // Check batch.error_file_id for details
    },
    Err(BatchWaitError::Timeout(duration)) => {
        println!("Timed out after {:?}", duration);
    },
    Err(BatchWaitError::Client(err)) => {
        eprintln!("Client error: {}", err);
    },
}
```

## Batch statuses

Inspect batch status directly via `retrieve_batch`:

```rust
let batch = client.retrieve_batch(&batch_id).await?;
println!("Status: {:?}", batch.status);
println!("Created: {}", batch.created_at);
println!("Completed at: {:?}", batch.completed_at);
```

## Retrieving results

When `status == Completed`:

```rust
if let Some(output_id) = batch.output_file_id {
    let output = client.retrieve_file_content(&output_id).await?;
    // JSONL stream of responses
}

if let Some(error_id) = batch.error_file_id {
    let errors = client.retrieve_file_content(&error_id).await?;
    // JSONL stream of request errors
}
```

Output files are in JSONL format: one response or error object per line. Parse
as needed for your application.
