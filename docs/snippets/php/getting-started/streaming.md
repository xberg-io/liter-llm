```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'Count from 1 to 5.']],
]));

foreach ($client->chatStream($request) as $chunkJson) {
    $chunk = json_decode($chunkJson, false, flags: JSON_THROW_ON_ERROR);
    echo $chunk->choices[0]->delta->content ?? '';
}
echo PHP_EOL;
```
