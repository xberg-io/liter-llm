```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'Explain quantum computing briefly']],
]));

$fullText = '';
foreach ($client->chatStream($request) as $chunkJson) {
    $chunk = json_decode($chunkJson, false, flags: JSON_THROW_ON_ERROR);
    $delta = $chunk->choices[0]->delta->content ?? null;
    if ($delta !== null) {
        $fullText .= $delta;
        echo $delta;
    }
}
echo PHP_EOL;
echo 'Full response length: ' . strlen($fullText) . ' characters' . PHP_EOL;
```
