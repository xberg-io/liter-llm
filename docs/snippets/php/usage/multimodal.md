```php
<?php

declare(strict_types=1);

use Liter\Llm\LiterLlm;
use Liter\Llm\Message;
use Liter\Llm\ContentPart;
use Liter\Llm\Image;
use Liter\Llm\ImageDetail;
use Liter\Llm\ChatCompletionRequest;

$client = LiterLlm::createClient(getenv('OPENAI_API_KEY') ?: '');

// Vision: Send an image and ask the model to analyze it
$response = $client->chat(
    ChatCompletionRequest::from_json(json_encode([
        'model' => 'gpt-4o',
        'messages' => [
            Message::userWithParts([
                ContentPart::text('What is in this image?'),
                ContentPart::imageUrl(
                    'https://upload.wikimedia.org/wikipedia/commons/thumb/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg/1280px-Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg',
                    ImageDetail::High
                ),
            ])->toArray(),
        ],
    ]))
);

echo "Analysis: " . $response->choices[0]->message->text() . PHP_EOL;

// Multimodal with base64 image
$imageBytes = file_get_contents('path/to/image.png');
$response = $client->chat(
    ChatCompletionRequest::from_json(json_encode([
        'model' => 'gpt-4o',
        'messages' => [
            Message::userWithParts([
                ContentPart::text('Describe what you see'),
                ContentPart::imagePng($imageBytes),
            ])->toArray(),
        ],
    ]))
);

echo $response->choices[0]->message->text() . PHP_EOL;

// Audio input (if supported by model)
$audioBytes = file_get_contents('path/to/audio.wav');
$audioBase64 = base64_encode($audioBytes);

$response = $client->chat(
    ChatCompletionRequest::from_json(json_encode([
        'model' => 'gpt-4o-audio-preview',
        'messages' => [
            Message::userWithParts([
                ContentPart::text('Transcribe this audio'),
                ContentPart::audio($audioBase64, 'wav'),
            ])->toArray(),
        ],
    ]))
);

echo "Transcription: " . $response->choices[0]->message->text() . PHP_EOL;

// Structured output with JSON schema
$response = $client->chat(
    ChatCompletionRequest::from_json(json_encode([
        'model' => 'gpt-4o',
        'messages' => [
            Message::user('Extract the person name and age from: "John Doe is 30 years old"'),
        ],
        'response_format' => [
            'type' => 'json_schema',
            'json_schema' => [
                'name' => 'person',
                'schema' => json_encode([
                    'type' => 'object',
                    'properties' => [
                        'name' => ['type' => 'string'],
                        'age' => ['type' => 'integer'],
                    ],
                    'required' => ['name', 'age'],
                ]),
                'strict' => true,
            ],
        ],
    ]))
);

$structured = json_decode($response->choices[0]->message->text(), true);
echo "Extracted: {$structured['name']}, age {$structured['age']}" . PHP_EOL;
```
