<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\CreateTranscriptionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$result = $client->transcribeAsync(new CreateTranscriptionRequest(
    model: 'openai/whisper-1',
    file: base64_encode(file_get_contents('audio.mp3')),
));

echo $result->text . PHP_EOL;
```
