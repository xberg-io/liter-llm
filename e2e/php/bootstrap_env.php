<?php

declare(strict_types=1);

// Hand-written wrapper bootstrap: loads .env from the liter-llm repo root
// before delegating to the alef-generated bootstrap.php so smoke tests
// gated on OPENAI_API_KEY/ANTHROPIC_API_KEY/GEMINI_API_KEY pick them up.

require_once __DIR__ . '/vendor/autoload.php';

if (class_exists(\Dotenv\Dotenv::class)) {
    $envDir = realpath(__DIR__ . '/../..');
    if ($envDir !== false && file_exists($envDir . '/.env')) {
        \Dotenv\Dotenv::createImmutable($envDir)->safeLoad();
    }
}

require_once __DIR__ . '/bootstrap.php';
