```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$messages = [
    ['role' => 'system', 'content' => 'You are a helpful assistant.'],
    ['role' => 'user', 'content' => 'What is the capital of France?'],
];

$result = $client->chatAsync(ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => $messages,
])));
$answer = $result->choices[0]->message->content;
echo "Assistant: {$answer}" . PHP_EOL;

$messages[] = ['role' => 'assistant', 'content' => $answer];
$messages[] = ['role' => 'user', 'content' => 'What about Germany?'];

$result = $client->chatAsync(ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => $messages,
])));
echo "Assistant: {$result->choices[0]->message->content}" . PHP_EOL;

if ($result->usage !== null) {
    echo "Tokens: {$result->usage->promptTokens} in, {$result->usage->completionTokens} out" . PHP_EOL;
}
```
