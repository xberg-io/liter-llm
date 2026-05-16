<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.util.ArrayList;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var messages = new ArrayList<Message>(List.of(
                new Message.System(new SystemMessage("You are a helpful assistant.", null)),
                new Message.User(new UserMessage(UserContent.of("What is the capital of France?"), null))
            ));

            var response = client.chat(ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o").withMessages(messages).build());
            var content = response.choices().getFirst().message().content();
            System.out.println("Assistant: " + content);

            messages.add(new Message.Assistant(new AssistantMessage(content, null, null, null, null)));
            messages.add(new Message.User(new UserMessage(UserContent.of("What about Germany?"), null)));

            response = client.chat(ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o").withMessages(messages).build());
            System.out.println("Assistant: " + response.choices().getFirst().message().content());

            var usage = response.usage();
            if (usage != null) {
                System.out.printf("Tokens: %d in, %d out%n",
                    usage.promptTokens(), usage.completionTokens());
            }
        }
    }
}
```
