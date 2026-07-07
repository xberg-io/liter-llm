<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\CreateSpeechRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

$audioBytes = $client->speechAsync(new CreateSpeechRequest(
    model: 'openai/tts-1',
    input: 'Hello, world!',
    voice: 'alloy',
));

// speechAsync returns the raw audio as an array of byte values.
$binary = pack('C*', ...$audioBytes);
file_put_contents('output.mp3', $binary);
echo 'Wrote ' . strlen($binary) . ' bytes to output.mp3' . PHP_EOL;
```
