<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;
use Liter\Llm\Message;
use Liter\Llm\UserMessage;
use Liter\Llm\UserContent;

// No API key needed for local providers
$client = LiterLlm::createClient(
    apiKey: "",
    baseUrl: "http://localhost:11434/v1"
);

$request = new ChatCompletionRequest(
    model: "ollama/qwen2:0.5b",
    messages: [
        new Message(
            role: "user",
            content: "Hello!",
            name: null
        )
    ]
);

$response = $client->chat($request);
echo $response->choices[0]->message->content . PHP_EOL;
```
