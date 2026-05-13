```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\EmbeddingRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = EmbeddingRequest::from_json(json_encode([
    'model' => 'openai/text-embedding-3-small',
    'input' => ['The quick brown fox jumps over the lazy dog'],
]));

$result = $client->embedAsync($request);
$embedding = $result->data[0]->embedding;
echo 'Dimensions: ' . count($embedding) . PHP_EOL;
echo 'First 5 values: ' . json_encode(array_slice($embedding, 0, 5)) . PHP_EOL;
```
