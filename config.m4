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
fi
