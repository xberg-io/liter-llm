<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\CreateBatchRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$result = $client->createBatchAsync(new CreateBatchRequest(
    inputFileId: 'file-abc123',
    endpoint: '/v1/chat/completions',
    completionWindow: '24h',
));

echo "Batch ID: {$result->id}" . PHP_EOL;
echo "Status: {$result->status}" . PHP_EOL;
```
