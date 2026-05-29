dnl Configuration for Rust-based PHP extension via ext-php-rs.
dnl Allows phpize to recognize this extension during source compilation (PIE fallback).

PHP_ARG_ENABLE([liter_llm],
  [whether to enable the liter_llm extension],
  [AS_HELP_STRING([--enable-liter_llm],
    [Enable liter_llm extension support])],
  [yes])

if test "$PHP_LITER_LLM_ENABLED" = "yes"; then
  dnl Recognize the extension directory for phpize/make
  PHP_NEW_EXTENSION(liter_llm, [], $ext_shared)

  dnl Invoke cargo build to compile the Rust FFI library
  AC_CONFIG_COMMANDS([cargo-build], [
    if test -f "crates/liter-llm-php/Cargo.toml"; then
      cargo build --release --manifest-path crates/liter-llm-php/Cargo.toml || exit 1
      cargo_output_dir="crates/liter-llm-php/target/release"
      ext_soname="liter_llm"

      dnl Detect output filename based on platform
      if test -f "${cargo_output_dir}/libliter-llm_php.dylib"; then
        cargo_lib="${cargo_output_dir}/libliter-llm_php.dylib"
      elif test -f "${cargo_output_dir}/libliter-llm_php.so"; then
        cargo_lib="${cargo_output_dir}/libliter-llm_php.so"
      else
        AC_MSG_ERROR([cargo build succeeded but .so/.dylib not found])
      fi

      dnl Copy the compiled library to modules/ directory for phpize to install
      cp "${cargo_lib}" "modules/${ext_soname}.so" || exit 1
    else
      AC_MSG_ERROR([crates/liter-llm-php/Cargo.toml not found])
    fi
  ], [
    extension_name=liter_llm
  ])
fi
