<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var response = client.createBatch(CreateBatchRequest.builder()
                .withInputFileId("file-abc123")
                .withEndpoint("/v1/chat/completions")
                .withCompletionWindow("24h")
                .build());
            System.out.println("Batch ID: " + response.id());
            System.out.println("Status: " + response.status());
        }
    }
}
```
