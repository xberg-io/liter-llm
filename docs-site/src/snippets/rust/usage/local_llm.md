```rust
use liter_llm::{
    ChatCompletionRequest, ClientConfigBuilder, DefaultClient, LlmClient,
    Message, UserContent, UserMessage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No API key needed for local providers
    let config = ClientConfigBuilder::new("")
        .base_url("http://localhost:11434/v1")
        .build();
    let client = DefaultClient::new(config, Some("ollama/qwen2:0.5b"))?;

    let request = ChatCompletionRequest {
        model: "ollama/qwen2:0.5b".into(),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello!".into()),
            name: None,
        })],
        ..Default::default()
    };

    let response = client.chat(request).await?;
    if let Some(choice) = response.choices.first() {
        println!("{}", choice.message.content.as_deref().unwrap_or(""));
    }
    Ok(())
}
```
