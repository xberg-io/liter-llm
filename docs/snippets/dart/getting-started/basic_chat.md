<!-- snippet:compile-only -->

```dart
import 'package:liter_llm/liter_llm.dart';
import 'dart:io';

void main() async {
  final client = await LiterLlmBridge.createClient(
    apiKey: Platform.environment['OPENAI_API_KEY'] ?? '',
  );
  final request = ChatCompletionRequest(
    model: 'openai/gpt-4o',
    messages: [Message.user(UserMessage(content: UserContent.of('Hello!')))],
  );
  final response = await client.chat(request);
  print(response.choices[0].message.content);
}
```
