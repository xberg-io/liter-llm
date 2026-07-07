<!-- snippet:compile-only -->

```rust
use base64::Engine;
use liter_llm::{ClientConfigBuilder, CreateFileRequest, DefaultClient, FileClient, FilePurpose};
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfigBuilder::new(std::env::var("OPENAI_API_KEY")?)
        .build();
    let client = DefaultClient::new(config, Some("openai/gpt-4o"))?;

    let bytes = fs::read("data.jsonl").await?;
    let response = client
        .create_file(CreateFileRequest {
            file: base64::engine::general_purpose::STANDARD.encode(&bytes),
            filename: Some("data.jsonl".into()),
            purpose: FilePurpose::Batch,
        })
        .await?;

    println!("File ID: {}", response.id);
    println!("Size: {} bytes", response.bytes);
    Ok(())
}
```
