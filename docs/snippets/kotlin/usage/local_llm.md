<!-- snippet:compile-only -->

```kotlin
import dev.kreuzberg.literllm.android.*
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val client = LiterLlm.createClient(apiKey = "", baseUrl = "http://localhost:11434/v1")
    val request = ChatCompletionRequest(
        model = "ollama/qwen2:0.5b",
        messages = listOf(Message.User(UserMessage(content = UserContent.of("Hello!"))))
    )
    val response = client.chat(request)
    println(response.choices[0].message.content)
}
```
