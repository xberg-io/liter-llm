#!/usr/bin/env bash
#MISE description="Run Go test_app smoke test"
#MISE dir="{{config_root}}/test_apps/go"
set -euo pipefail

export GOWORK=off
# `alef sync-versions` rewrites go.mod but not go.sum, so `go mod download` alone leaves
# the published module without a checksum; tidy first, as `alef test-apps run` does. ~keep
go mod tidy
# Go selects tests by name, not file; derive the filter from the generated smoke_test.go
# so regeneration cannot leave a hard-coded prefix matching a subset or nothing. ~keep
go test -v -run "$(python3 "$MISE_CONFIG_ROOT/scripts/ci/go_smoke_test_filter.py" smoke_test.go)"
