<!-- snippet:compile-only -->

```swift
import Foundation
import LiterLlm

let client = try await LiterLlm.createClient(apiKey: ProcessInfo.processInfo.environment["OPENAI_API_KEY"] ?? "")
let request = ChatCompletionRequest(
    model: "openai/gpt-4o",
    messages: [.user(.init(content: .of("Hello!")))],
    temperature: nil, topP: nil, maxTokens: nil, toolChoice: nil, tools: nil, responseFormat: nil
)
let response = try await client.chat(request)
print(response.choices[0].message.content ?? "")
```
