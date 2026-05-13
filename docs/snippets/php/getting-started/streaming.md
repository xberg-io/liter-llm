```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'Count from 1 to 5.']],
    'stream' => true,
]));

// chatStreamAsync collects the full stream and returns it as a JSON-encoded
// array of chunks. Decode and iterate.
$chunks = json_decode($client->chatStreamAsync($request), true);
foreach ($chunks as $chunk) {
    echo $chunk['choices'][0]['delta']['content'] ?? '';
}
echo PHP_EOL;
```
