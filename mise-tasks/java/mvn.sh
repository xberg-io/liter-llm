#!/usr/bin/env bash
#MISE description="Run Maven in packages/java with the sdkman JDK and Maven when sdkman is installed"
#MISE hide=true
#MISE dir="{{config_root}}/packages/java"
set -euo pipefail

# sdkman scripts read unset variables, so they run without nounset. ~keep
if [ -f "$HOME/.sdkman/bin/sdkman-init.sh" ]; then
  set +u
  # shellcheck disable=SC1091 # sdkman is a per-machine install, not part of this repository
  source "$HOME/.sdkman/bin/sdkman-init.sh"
  sdk use java 25.0.2-tem
  sdk use maven 3.9.11
  set -u
fi
mvn "$@"
