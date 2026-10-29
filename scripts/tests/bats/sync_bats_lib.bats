#!/usr/bin/env bats
#
# Contract tests for scripts/task/sync-bats-lib.sh.
#
# curl is a stub that records the URL it was asked for and writes a canned library body to the
# --output target, so no test touches the network. BATS_LIB_DEST points at a scratch directory;
# the vendored library in the repository is never read or written. ~keep

setup() {
	bats_load_library xberg-bats
	xberg_setup

	ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$ROOT/scripts/task/sync-bats-lib.sh"
	export BATS_LIB_DEST="$XBERG_WORK/vendored"
	unset BATS_LIB_REF

	export LIB_BODY="# canned helper library"
	xberg_stub curl \
		'target=""' \
		'for ((i = 1; i <= $#; i++)); do' \
		'  case "${!i}" in' \
		'    --output) next=$((i + 1)); target="${!next}" ;;' \
		'  esac' \
		'done' \
		'printf "%s\n" "${!#}" >>"$XBERG_TRACE"' \
		'printf "%s\n" "$LIB_BODY" >"$target"'
}

@test "sync-bats-lib should_require_a_pinned_ref" {
	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "BATS_LIB_REF must name an immutable xberg-io/actions tag or commit."
	xberg_assert_trace_empty
}

@test "sync-bats-lib should_reject_a_branch_name_as_the_ref" {
	export BATS_LIB_REF="main"

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "not 'main'"
	xberg_assert_trace_empty
}

@test "sync-bats-lib should_reject_a_short_commit_sha_as_the_ref" {
	export BATS_LIB_REF="0123abc"

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "not '0123abc'"
}

@test "sync-bats-lib should_vendor_the_library_from_the_pinned_tag" {
	export BATS_LIB_REF="v1.2.3"

	run bash "$SCRIPT" sync

	xberg_assert_status 0
	xberg_assert_output_contains "xberg-io/actions@v1.2.3"
	xberg_assert_trace "https://raw.githubusercontent.com/xberg-io/actions/v1.2.3/bats-lib/xberg-bats/load.bash"
	xberg_assert_file "$BATS_LIB_DEST/load.bash" "$LIB_BODY"
}

@test "sync-bats-lib should_accept_a_full_commit_sha_as_the_ref" {
	export BATS_LIB_REF="0123456789abcdef0123456789abcdef01234567"

	run bash "$SCRIPT" sync

	xberg_assert_status 0
	xberg_assert_trace "https://raw.githubusercontent.com/xberg-io/actions/0123456789abcdef0123456789abcdef01234567/bats-lib/xberg-bats/load.bash"
}

@test "sync-bats-lib should_default_to_sync_when_no_mode_is_given" {
	export BATS_LIB_REF="v1.2.3"

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_file "$BATS_LIB_DEST/load.bash" "$LIB_BODY"
}

@test "sync-bats-lib should_pass_the_check_when_the_vendored_copy_matches" {
	export BATS_LIB_REF="v1.2.3"
	mkdir -p "$BATS_LIB_DEST"
	printf '%s\n' "$LIB_BODY" >"$BATS_LIB_DEST/load.bash"

	run bash "$SCRIPT" --check

	xberg_assert_status 0
	xberg_assert_output_contains "matches xberg-io/actions@v1.2.3"
}

@test "sync-bats-lib should_fail_the_check_when_the_vendored_copy_has_drifted" {
	export BATS_LIB_REF="v1.2.3"
	mkdir -p "$BATS_LIB_DEST"
	printf '%s\n' "# an older copy" >"$BATS_LIB_DEST/load.bash"

	run bash "$SCRIPT" --check

	xberg_assert_status 1
	xberg_assert_output_contains "has drifted from xberg-io/actions@v1.2.3"
	xberg_assert_output_contains "mise run test:scripts:lib:sync"
	xberg_assert_file "$BATS_LIB_DEST/load.bash" "# an older copy"
}

@test "sync-bats-lib should_reject_an_unknown_mode" {
	export BATS_LIB_REF="v1.2.3"

	run bash "$SCRIPT" refresh

	xberg_assert_status 2
	xberg_assert_output_contains "usage:"
	xberg_assert_file_absent "$BATS_LIB_DEST/load.bash"
}
