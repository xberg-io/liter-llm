<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\CreateFileRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$result = $client->createFileAsync(new CreateFileRequest(
    file: base64_encode(file_get_contents('data.jsonl')),
    purpose: 'batch',
    filename: 'data.jsonl',
));

echo "File ID: {$result->id}" . PHP_EOL;
echo "Size: {$result->bytes} bytes" . PHP_EOL;
```
