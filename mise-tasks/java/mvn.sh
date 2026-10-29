#!/usr/bin/env bash
#MISE description="Run Maven in packages/java with the JDK and Maven that mise.toml pins"
#MISE hide=true
#MISE dir="{{config_root}}/packages/java"
set -euo pipefail

mvn "$@"
