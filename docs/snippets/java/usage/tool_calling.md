<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var tools = List.of(new ChatCompletionTool(
                ToolType.Function,
                new FunctionDefinition(
                    "get_weather",
                    "Get the current weather for a location",
                    Map.of(
                        "type", "object",
                        "properties", Map.of(
                            "location", Map.of("type", "string", "description", "City name")
                        ),
                        "required", List.of("location")
                    ),
                    null
                )
            ));

            var request = ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o")
                .withMessages(List.of(
                    new Message.User(new UserMessage(UserContent.of("What is the weather in Berlin?"), null))
                ))
                .withTools(Optional.of(tools))
                .build();
            var response = client.chat(request);

            var toolCalls = response.choices().getFirst().message().toolCalls();
            if (toolCalls != null) {
                for (var call : toolCalls) {
                    System.out.printf("Tool: %s, Args: %s%n",
                        call.function().name(), call.function().arguments());
                }
            }
        }
    }
}
```
