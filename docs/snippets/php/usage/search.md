<!-- snippet:compile-only -->

```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\SearchRequest;

$client = LiterLlm::createClient(getenv('BRAVE_API_KEY') ?: '');

$result = $client->searchAsync(new SearchRequest(
    model: 'brave/web-search',
    query: 'What is Rust programming language?',
    maxResults: 5,
));

foreach ($result->results as $r) {
    echo "{$r->title}: {$r->url}" . PHP_EOL;
}
```
