<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\RerankRequest;

$client = LiterLlm::createClient(getenv('COHERE_API_KEY') ?: '');

$request = RerankRequest::from_json(json_encode([
    'model' => 'cohere/rerank-v3.5',
    'query' => 'What is the capital of France?',
    'documents' => [
        'Paris is the capital of France.',
        'Berlin is the capital of Germany.',
        'London is the capital of England.',
    ],
]));

$result = $client->rerankAsync($request);
foreach ($result->results as $r) {
    echo "Index: {$r->index}, Score: " . number_format($r->relevanceScore, 4) . PHP_EOL;
}
```
