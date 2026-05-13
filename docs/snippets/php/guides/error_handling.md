```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;
use Liter\Llm\LiterLlmException;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'Hello']],
]));

try {
    $result = $client->chatAsync($request);
    echo $result->choices[0]->message->content . PHP_EOL;
} catch (LiterLlmException $e) {
    // All liter-llm errors surface as a single LiterLlmException type.
    // The exception message is the Rust error's Display string — branch on it
    // to identify the category.
    $msg = $e->getMessage();
    if (stripos($msg, 'authentication') !== false) {
        fwrite(STDERR, "auth failed: $msg\n");
    } elseif (stripos($msg, 'rate limit') !== false) {
        fwrite(STDERR, "rate limited: $msg\n");
    } elseif (stripos($msg, 'context window') !== false) {
        fwrite(STDERR, "prompt too long: $msg\n");
    } else {
        fwrite(STDERR, "llm error: $msg\n");
    }
}
```
