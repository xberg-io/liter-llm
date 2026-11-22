# liter-llm-cli

CLI proxy for [`liter-llm`](https://github.com/xberg-io/liter-llm). Installing
this package provides a `liter-llm` command.

The package version always equals the `liter-llm` release it carries. Platform
wheels bundle the native binary; the source distribution instead downloads
exactly `liter-llm-<version>-<target>` from the matching `v<version>` GitHub
release and verifies it against `SHA256SUMS-<version>.txt` before running it.
Set `LITER_LLM_BINARY` to run a binary you already have.
