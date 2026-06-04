dnl Configuration for Rust-based PHP extension via ext-php-rs.
dnl Allows phpize to recognize this extension during source compilation (PIE fallback).

PHP_ARG_ENABLE([liter_llm],
  [whether to enable the liter_llm extension],
  [AS_HELP_STRING([--enable-liter_llm],
    [Enable liter_llm extension support])],
  [yes])

if test "$PHP_LITER_LLM_ENABLED" = "yes"; then
  dnl Register the extension directory so phpize creates modules/ and sets up build rules.
  PHP_NEW_EXTENSION(liter_llm, [], $ext_shared)

  dnl Invoke cargo build to compile the Rust FFI library and copy it to modules/.
  AC_CONFIG_COMMANDS([cargo-build], [
    if test -f "crates/liter-llm-php/Cargo.toml"; then
      (cd crates/liter-llm-php && cargo build --release) || exit 1

      dnl Detect output filename based on platform
      if test -f "crates/liter-llm-php/target/release/libliter-llm_php.dylib"; then
        cargo_lib="crates/liter-llm-php/target/release/libliter-llm_php.dylib"
      elif test -f "crates/liter-llm-php/target/release/libliter-llm_php.so"; then
        cargo_lib="crates/liter-llm-php/target/release/libliter-llm_php.so"
      else
        echo "ERROR: cargo build succeeded but .so/.dylib not found in crates/liter_llm-php/target/release" >&2
        exit 1
      fi

      mkdir -p modules
      cp "$cargo_lib" "modules/liter-llm.so" || exit 1
    else
      echo "ERROR: crates/liter_llm-php/Cargo.toml not found" >&2
      exit 1
    fi
  ], [])
fi
