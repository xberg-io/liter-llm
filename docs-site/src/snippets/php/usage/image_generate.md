<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\CreateImageRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$result = $client->imageGenerateAsync(new CreateImageRequest(
    prompt: 'A sunset over mountains',
    model: 'openai/dall-e-3',
    n: 1,
    size: '1024x1024',
));

echo $result->data[0]->url . PHP_EOL;
```
