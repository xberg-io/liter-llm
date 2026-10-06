---
id: readme_swift_basic_chat
language: swift
target: swift
level: syntax
requires: []
side_effect: network
---

Send a message to any provider using the `provider/model` prefix.

```swift
import Foundation
import LiterLlm

let client = try LiterLlm.createClient(apiKey: ProcessInfo.processInfo.environment["OPENAI_API_KEY"] ?? "")
let request = try LiterLlm.chatCompletionRequestFromJson(
    #"{"model": "openai/gpt-4o", "messages": [{"role": "user", "content": "Hello!"}]}"#
)
let response = try await client.chat(request)
print(response.choices()[0].message().content()?.toString() ?? "")
```
