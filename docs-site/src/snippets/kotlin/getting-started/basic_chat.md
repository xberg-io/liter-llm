<!-- snippet:compile-only -->

```kotlin
import io.xberg.literllm.android.*
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY") ?: "")
    val request = ChatCompletionRequest(
        model = "openai/gpt-4o",
        messages = listOf(Message.User(UserMessage(content = UserContent.of("Hello!"))))
    )
    val response = client.chat(request)
    println(response.choices[0].message.content)
}
```
