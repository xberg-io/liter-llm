#!/usr/bin/env bash
#MISE description="Install and configure sdkman for JVM toolchain management (run separately if you need Java/Maven)"
set -euo pipefail

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
  echo "Setting up sdkman (skipped on Windows - use scoop or manual installation)..."
  exit 0
  ;;
esac
echo "Setting up sdkman..."
if [ ! -d "$HOME/.sdkman" ]; then
  echo "Installing sdkman..."
  curl -s "https://get.sdkman.io" | bash
else
  echo "sdkman is already installed"
fi
echo "Loading sdkman and installing tools from .sdkmanrc..."
set +u
# shellcheck disable=SC1091 # installed above, outside this repository
source "$HOME/.sdkman/bin/sdkman-init.sh"
sdk env install
