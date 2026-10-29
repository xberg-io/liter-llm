---
description: "Contributing to liter-llm"
title: "Contributing to liter-llm"
---

Thank you for your interest in contributing to liter-llm! This guide will help you get started with development.

## Development Setup

### mise Installation

This project uses [mise](https://mise.jdx.dev/) for its toolchain and task automation. `mise.toml` pins every tool version, and `mise run <task>` runs the repository tasks with those tools on PATH.

Install mise by following the [official getting-started guide](https://mise.jdx.dev/getting-started.html), then trust the repository configuration once:

```bash
mise trust
```

### Quick Start

After installing mise, set up your development environment:

```bash
# One-time setup - installs all dependencies
mise run setup

# Build the core Rust library
mise run build
```

The setup command will install Rust, Python, Node.js, Go, Java, and Elixir tooling as needed.

## Development Workflow

### Common Commands

```bash
# Build all crates
mise run build

# Build in release mode (optimized)
mise run build:release
```

```bash
# Run all tests
mise run test

# Run all checks (lint + test)
mise run check
```

```bash
# Format all code
mise run format

# Run all linters via prek
mise run lint

# Generate READMEs from templates
mise run generate:readme
```

```bash
# Update all dependencies
mise run update

# Clean all build artifacts
mise run clean
```

### Language-Specific Tasks

Each language binding has its own namespace. Format and lint every language with the root `mise run format` and `mise run lint`.

**Rust:**

```bash
mise run rust:build
mise run rust:test
mise run rust:format
mise run rust:lint
```

**Python:**

```bash
mise run python:install
mise run python:build
```

**Node.js:**

```bash
mise run node:build        # Build NAPI-RS native module (release)
mise run node:build:dev    # Build in debug mode
mise run node:test
```

**Go:**

```bash
mise run go:build          # Build Go bindings (requires FFI)
mise run go:build:ffi      # Build FFI static library for Go
mise run go:test
mise run go:lint
```

**Java:**

```bash
mise run java:test
```

**Elixir:**

```bash
mise run elixir:build      # Compile (includes Rustler NIF)
mise run elixir:test
mise run elixir:deps
```

**Ruby:**

```bash
mise run ruby:build        # Build Ruby native extension
mise run ruby:test         # Run Ruby tests
```

**WebAssembly:**

```bash
mise run wasm:build         # Build WASM package (web target)
mise run wasm:build:bundler # Build WASM package (bundler target)
mise run wasm:build:node    # Build WASM package (Node.js target)
mise run wasm:test          # Run WASM tests
```

**C:**

```bash
mise run c:build:ffi       # Build FFI library for C tests
mise run c:e2e:test        # Run C E2E tests
```

## Adding Providers

### Steps

1. **Add a provider entry** to `schemas/providers.json`:

   ```json
   {
     "my-provider": {
       "base_url": "https://api.myprovider.com/v1",
       "auth_header": "Authorization",
       "auth_prefix": "Bearer",
       "model_prefixes": ["my-provider/"],
       "parameter_mappings": {}
     }
   }
   ```

   Fields:
   - `base_url` (required): Provider API base URL
   - `auth_header` (required): Header name for authentication
   - `auth_prefix` (optional): Prefix for the auth value (e.g. "Bearer")
   - `model_prefixes` (required): Model name prefixes that route to this provider
   - `parameter_mappings` (optional): Map OpenAI parameter names to provider-specific names

1. **Build and test**

   ```bash
   mise run build
   mise run test
   ```

1. **Regenerate E2E tests**

   ```bash
   mise run e2e:generate:all
   mise run test
   ```

## E2E Tests

E2E tests are generated from JSON fixtures in `tools/e2e-generator/fixtures/` and produce runnable test suites for each language binding.

```bash
# Generate E2E tests for all languages
mise run e2e:generate

# Run Rust E2E tests
mise run rust:e2e:test
```

Generated test files in `e2e/` should not be edited directly -- modify fixtures or the generator source instead.

## Exploring Tasks

```bash
# Show all available tasks
mise tasks

# Show all tasks including hidden ones
mise tasks --hidden
```

## Code Quality

### Pre-commit Hooks

The project uses [prek](https://github.com/j178/prek) for pre-commit hooks:

```bash
# Install hooks
prek install
prek install --hook-type commit-msg

# Run all hooks manually
prek run --all-files
```

### Commit Messages

We use conventional commits:

- `feat: add support for new-provider`
- `fix: correct auth header injection`
- `docs: update installation instructions`
- `chore: update dependencies`
- `test: add tests for streaming`

## Submitting Changes

1. **Create a feature branch**

   ```bash
   git checkout -b feat/add-provider-support
   ```

1. **Make your changes** and run checks locally:

   ```bash
   mise run check
   ```

1. **Commit and push**

   ```bash
   git commit -m "feat: add support for new provider"
   git push origin feat/add-provider-support
   ```

1. **Create a Pull Request** -- link any related issues and ensure CI passes.

## Maintenance Tasks

### Version Synchronization

Version is managed in `Cargo.toml` workspace and synced across all manifests:

```bash
mise run version:sync
```

## Questions?

- Check existing [issues](https://github.com/xberg-io/liter-llm/issues)
- Join our [Discord community](https://discord.gg/xt9WY3GnKR)
