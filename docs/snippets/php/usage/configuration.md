```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

// createClient signature:
// LiterLlm::createClient(string $apiKey, ?string $baseUrl = null,
//                       ?int $timeoutSecs = null, ?int $maxRetries = null,
//                       ?string $modelHint = null): DefaultClient
$client = LiterLlm::createClient(
    getenv('OPENAI_API_KEY') ?: '',
    null,        // baseUrl — override provider base URL
    60,          // timeoutSecs
    3,           // maxRetries
    'openai',    // modelHint — pre-resolve provider
);

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'Hello!']],
]));

$result = $client->chatAsync($request);
echo $result->choices[0]->message->content . PHP_EOL;
```
