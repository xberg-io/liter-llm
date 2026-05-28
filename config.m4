dnl config.m4 for liter_llm extension

PHP_ARG_ENABLE(liter_llm, whether to enable liter_llm support,
[  --enable-liter_llm      Enable liter_llm support])

if test "$PHP_LITER_LLM" != "no"; then
  PHP_NEW_EXTENSION(liter_llm, , $ext_shared)
fi
