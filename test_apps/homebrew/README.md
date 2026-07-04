# homebrew test_app

Exercises the configured Homebrew formulae from tap `xberg-io/homebrew-tap` at version `1.9.3`.

| Formula | Purpose |
|---------|--------|
| `liter-llm` | CLI binary |

## Running

```bash
bash run_tests.sh
```

## What it tests

1. `brew bundle install` succeeds (tap + formulae install without error).
2. `$CLI_FORMULA --version` — output contains `$VERSION`.
3. `$CLI_FORMULA --help` — output contains `Usage`.
