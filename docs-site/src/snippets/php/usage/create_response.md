<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\CreateResponseRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = CreateResponseRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o',
    'input' => 'Explain quantum computing in one sentence.',
]));

$result = $client->createResponseAsync($request);
echo "Response ID: {$result->id}" . PHP_EOL;
echo "Status: {$result->status}" . PHP_EOL;
foreach ($result->output as $item) {
    echo $item->content . PHP_EOL;
}
```
