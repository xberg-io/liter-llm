#!/usr/bin/env bash
#MISE description="Run the Kotlin/Android test_app host JVM tests with a JDK the Android Gradle plugin accepts"
#MISE hide=true
#MISE dir="{{config_root}}/test_apps/kotlin_android"
#USAGE arg "<suite>" help="Suite name for the skip messages: smoke or comprehensive"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

suite="$usage_suite"
if [ -z "${ANDROID_HOME:-}${ANDROID_SDK_ROOT:-}" ]; then
  echo "SKIP: kotlin_android $suite requires ANDROID_HOME or ANDROID_SDK_ROOT"
  exit 0
fi

# Detect a compatible JDK (17 or 21). AGP 8.13.0 rejects JDK 25+ as the gradle daemon.
# The build.gradle.kts jvmToolchain(17) pins compilation JDK but NOT the gradle daemon,
# so we must override JAVA_HOME if the host JDK is too new. ~keep
detect_jdk() {
  local dir java_path
  if [ "$(uname)" = "Darwin" ]; then
    # macOS: try homebrew's openjdk@17 and openjdk@21 symlinks first,
    # then fall back to /usr/libexec/java_home (which may return newer versions). ~keep
    for dir in /opt/homebrew/opt/openjdk@17 /opt/homebrew/opt/openjdk@21; do
      if [ -d "$dir" ]; then
        echo "$dir"
        return 0
      fi
    done
    /usr/libexec/java_home -v17 2>/dev/null || /usr/libexec/java_home -v21 2>/dev/null
  elif [ "$(uname)" = "Linux" ]; then
    if command -v update-alternatives >/dev/null 2>&1; then
      java_path=$(update-alternatives --list java 2>/dev/null | grep -E 'java-17|java-21|openjdk-17|openjdk-21' | head -1 || true)
      if [ -n "$java_path" ]; then
        dirname "$(dirname "$java_path")"
        return 0
      fi
    fi
    for dir in /usr/lib/jvm/java-17-openjdk* /usr/lib/jvm/java-21-openjdk* /usr/lib/jvm/temurin-17 /usr/lib/jvm/temurin-21; do
      if [ -d "$dir" ]; then
        echo "$dir"
        return 0
      fi
    done
  fi
  return 1
}

jdk_home=$(detect_jdk || true)
if [ -n "$jdk_home" ]; then
  export JAVA_HOME="$jdk_home"
  export PATH="$jdk_home/bin:$PATH"
else
  echo "SKIP: kotlin_android $suite requires JDK 17 or 21. Install with: brew install openjdk@17 (macOS) or sdk install java 17.0.13-zulu (sdkman)"
  exit 0
fi

./gradlew test
