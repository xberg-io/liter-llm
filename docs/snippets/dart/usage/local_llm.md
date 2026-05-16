<!-- snippet:compile-only -->

```dart
import 'package:liter_llm/liter_llm.dart';

void main() async {
  final client = await LiterLlmBridge.createClient(
    apiKey: '',
    baseUrl: 'http://localhost:11434/v1',
  );
  final request = ChatCompletionRequest(
    model: 'ollama/qwen2:0.5b',
    messages: [Message.user(UserMessage(content: UserContent.of('Hello!')))],
  );
  final response = await client.chat(request);
  print(response.choices[0].message.content);
}
```
