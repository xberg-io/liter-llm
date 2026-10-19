<?php

// Local smoke check for a built liter_llm PHP extension: it must load, report
// the expected version, expose the core classes and construct a client. It
// sends no requests. Run through scripts/php/smoke.sh, which loads the built
// extension and passes the expected version as the first argument.

declare(strict_types=1);

function smoke_fail(string $message): never
{
    fwrite(STDERR, "php smoke: {$message}\n");
    exit(1);
}

$expectedVersion = $argv[1] ?? '';

if (!extension_loaded('liter_llm')) {
    smoke_fail('the liter_llm extension is not loaded');
}

$version = phpversion('liter_llm');
if ($expectedVersion !== '' && $version !== $expectedVersion) {
    smoke_fail("extension version {$version} does not match the workspace version {$expectedVersion}");
}

foreach (['Liter\\Llm\\LiterLlmApi', 'Liter\\Llm\\DefaultClient', 'Liter\\Llm\\ChatCompletionRequest'] as $class) {
    if (!class_exists($class)) {
        smoke_fail("missing class {$class}");
    }
}

try {
    $client = \Liter\Llm\LiterLlmApi::createClient('smoke-test-key', 'http://127.0.0.1:9');
} catch (\Throwable $e) {
    smoke_fail('createClient failed: ' . $e->getMessage());
}
if (!$client instanceof \Liter\Llm\DefaultClient) {
    smoke_fail('createClient did not return a DefaultClient');
}

fwrite(STDOUT, "php smoke: liter_llm {$version} OK\n");
