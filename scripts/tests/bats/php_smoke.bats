#!/usr/bin/env bats
#
# Contract tests for scripts/php/smoke.sh.
#
# PHP is a stub that records its argv, so the tests assert the exact command the script hands to
# the interpreter (`exec`, so the interpreter's exit status is the script's). The script reads
# the version from Cargo.toml with python3, which stays real. ~keep

setup() {
	bats_load_library xberg-bats
	xberg_setup
	python3 -c 'import tomllib' 2>/dev/null || skip "python3 with tomllib is required"

	SOURCE="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)/scripts/php/smoke.sh"
	REPO="$XBERG_WORK/repo"
	SCRIPT="$REPO/scripts/php/smoke.sh"
	mkdir -p "$(dirname "$SCRIPT")" "$REPO/target/release"
	cp "$SOURCE" "$SCRIPT"
	printf '%s\n' '[workspace.package]' 'version = "9.8.7"' >"$REPO/Cargo.toml"

	xberg_stub php \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'exit "${PHP_STATUS:-0}"'
	export PHP="$XBERG_STUB_BIN/php"
	unset LITER_LLM_PHP_EXT
}

@test "php-smoke should_fail_when_no_extension_can_be_found" {
	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "liter_llm extension not found; run \`task php:build\`"
	xberg_assert_trace_empty
}

@test "php-smoke should_name_the_configured_extension_path_when_it_does_not_exist" {
	export LITER_LLM_PHP_EXT="$XBERG_WORK/absent.so"

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "liter_llm extension not found at $XBERG_WORK/absent.so"
	xberg_assert_trace_empty
}

@test "php-smoke should_run_the_smoke_check_against_the_configured_extension" {
	export LITER_LLM_PHP_EXT="$XBERG_WORK/custom.so"
	: >"$LITER_LLM_PHP_EXT"

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_trace "-n -d extension=$XBERG_WORK/custom.so $REPO/scripts/php/smoke.php 9.8.7"
}

@test "php-smoke should_use_the_release_build_when_no_extension_is_configured" {
	: >"$REPO/target/release/libliter_llm_php.so"

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_trace "-n -d extension=$REPO/target/release/libliter_llm_php.so $REPO/scripts/php/smoke.php 9.8.7"
}

@test "php-smoke should_prefer_the_configured_extension_over_the_release_build" {
	: >"$REPO/target/release/libliter_llm_php.so"
	export LITER_LLM_PHP_EXT="$XBERG_WORK/custom.so"
	: >"$LITER_LLM_PHP_EXT"

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_trace "-n -d extension=$XBERG_WORK/custom.so $REPO/scripts/php/smoke.php 9.8.7"
}

@test "php-smoke should_return_the_exit_status_of_the_interpreter" {
	export LITER_LLM_PHP_EXT="$XBERG_WORK/custom.so"
	: >"$LITER_LLM_PHP_EXT"
	export PHP_STATUS=3

	run bash "$SCRIPT"

	xberg_assert_status 3
}
