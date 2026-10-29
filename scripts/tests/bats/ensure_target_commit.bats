#!/usr/bin/env bats
#
# Contract tests for scripts/publish/common/ensure-target-commit.sh.
#
# git is a stub that records its argv. The trace is the evidence that the script either did
# nothing or checked out exactly the commit it was given. ~keep

setup() {
	bats_load_library xberg-bats
	xberg_setup

	ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$ROOT/scripts/publish/common/ensure-target-commit.sh"
	xberg_stub git \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'exit "${GIT_STATUS:-0}"'
	unset TARGET_SHA
}

@test "ensure-target-commit should_do_nothing_when_no_target_is_set" {
	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_no_output
	xberg_assert_trace_empty
}

@test "ensure-target-commit should_do_nothing_when_the_target_is_empty" {
	export TARGET_SHA=""

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_trace_empty
}

@test "ensure-target-commit should_force_checkout_the_target_commit" {
	export TARGET_SHA="0123456789abcdef0123456789abcdef01234567"

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_trace "checkout --progress --force 0123456789abcdef0123456789abcdef01234567"
}

@test "ensure-target-commit should_fail_when_the_checkout_fails" {
	export TARGET_SHA="0123456789abcdef0123456789abcdef01234567"
	export GIT_STATUS=128

	run bash "$SCRIPT"

	xberg_assert_status 128
}
