<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\OcrRequest;

$client = LiterLlm::createClient(getenv('MISTRAL_API_KEY') ?: '');

$request = OcrRequest::from_json(json_encode([
    'model' => 'mistral/mistral-ocr-latest',
    'document' => ['type' => 'document_url', 'url' => 'https://example.com/invoice.pdf'],
]));

$result = $client->ocrAsync($request);
foreach ($result->pages as $page) {
    echo "Page {$page->index}: " . substr($page->markdown, 0, 100) . '...' . PHP_EOL;
}
```
