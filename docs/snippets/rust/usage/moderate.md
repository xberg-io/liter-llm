<!-- snippet:compile-only -->

```rust
use liter_llm::{ClientConfigBuilder, DefaultClient, LlmClient, ModerationInput, ModerationRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfigBuilder::new(std::env::var("OPENAI_API_KEY")?)
        .build();
    let client = DefaultClient::new(config, Some("openai/omni-moderation-latest"))?;

    let response = client
        .moderate(ModerationRequest {
            model: Some("openai/omni-moderation-latest".into()),
            input: ModerationInput::Single("This is a test message.".into()),
        })
        .await?;

    let result = &response.results[0];
    println!("Flagged: {}", result.flagged);
    if result.categories.sexual {
        println!("  sexual: {:.4}", result.category_scores.sexual);
    }
    if result.categories.hate {
        println!("  hate: {:.4}", result.category_scores.hate);
    }
    if result.categories.self_harm {
        println!("  self-harm: {:.4}", result.category_scores.self_harm);
    }
    if result.categories.violence {
        println!("  violence: {:.4}", result.category_scores.violence);
    }
    Ok(())
}
```
