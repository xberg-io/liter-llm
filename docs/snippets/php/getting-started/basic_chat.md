```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'Hello!']],
]));

$result = $client->chatAsync($request);
echo $result->choices[0]->message->content . PHP_EOL;
```
