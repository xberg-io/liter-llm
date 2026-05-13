```java
import dev.kreuzberg.literllm.*;
import java.util.List;

public class ErrorHandling {
    public static void main(String[] args) {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var response = client.chat(ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o")
                .withMessages(List.of(
                    new Message.User(new UserMessage(UserContent.of("Hello"), null))
                ))
                .build());
            System.out.println(response.choices().get(0).message().content());
        } catch (AuthenticationException e) {
            // 401/403 — rotate the key.
            System.err.println("auth failed: " + e.getMessage());
        } catch (RateLimitedException e) {
            // 429 — transient, retry with backoff.
            System.err.println("rate limited: " + e.getMessage());
        } catch (BudgetExceededException e) {
            System.err.println("budget exceeded: " + e.getMessage());
        } catch (ServerErrorException | ServiceUnavailableException e) {
            // 5xx — usually transient.
            System.err.println("server error: " + e.getMessage());
        } catch (EndpointNotSupportedException e) {
            System.err.println("endpoint not supported by provider: " + e.getMessage());
        } catch (LiterLlmErrorException e) {
            // Catch-all for typed liter-llm errors.
            System.err.println("llm error: " + e.getMessage());
        } catch (LiterLlmRsException e) {
            // FFI-level error (carries a numeric code).
            System.err.println("ffi error (" + e.getCode() + "): " + e.getMessage());
        } catch (Exception e) {
            System.err.println("unexpected: " + e.getMessage());
        }
    }
}
```
