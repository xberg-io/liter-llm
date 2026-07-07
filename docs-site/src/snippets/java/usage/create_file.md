<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Base64;
import java.util.Optional;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            byte[] fileBytes = Files.readAllBytes(Path.of("data.jsonl"));
            String fileBase64 = Base64.getEncoder().encodeToString(fileBytes);
            var response = client.createFile(CreateFileRequest.builder()
                .withFile(fileBase64)
                .withPurpose(FilePurpose.Batch)
                .withFilename(Optional.of("data.jsonl"))
                .build());
            System.out.println("File ID: " + response.id());
            System.out.println("Size: " + response.bytes() + " bytes");
        }
    }
}
```
