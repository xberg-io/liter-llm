```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$tools = [[
    'type' => 'function',
    'function' => [
        'name' => 'get_weather',
        'description' => 'Get the current weather for a location',
        'parameters' => [
            'type' => 'object',
            'properties' => ['location' => ['type' => 'string', 'description' => 'City name']],
            'required' => ['location'],
        ],
    ],
]];

$request = ChatCompletionRequest::from_json(json_encode([
    'model' => 'openai/gpt-4o-mini',
    'messages' => [['role' => 'user', 'content' => 'What is the weather in Berlin?']],
    'tools' => $tools,
    'tool_choice' => 'auto',
]));

$result = $client->chat($request);
foreach ($result->choices[0]->message->toolCalls ?? [] as $call) {
    echo "Tool: {$call->function->name}, Args: {$call->function->arguments}" . PHP_EOL;
}
```
