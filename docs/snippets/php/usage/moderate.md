<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ModerationRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = ModerationRequest::from_json(json_encode([
    'model' => 'openai/omni-moderation-latest',
    'input' => 'This is a test message.',
]));

$result = $client->moderateAsync($request);
$first = $result->results[0];
echo 'Flagged: ' . ($first->flagged ? 'true' : 'false') . PHP_EOL;
```
