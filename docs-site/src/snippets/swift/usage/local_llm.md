<!-- snippet:compile-only -->

```swift
import Foundation
import LiterLlm

let client = try await LiterLlm.createClient(
    apiKey: "",
    baseUrl: "http://localhost:11434/v1"
)
let request = ChatCompletionRequest(
    model: "ollama/qwen2:0.5b",
    messages: [.user(.init(content: .of("Hello!")))],
    temperature: nil, topP: nil, maxTokens: nil, toolChoice: nil, tools: nil, responseFormat: nil
)
let response = try await client.chat(request)
print(response.choices[0].message.content ?? "")
```
