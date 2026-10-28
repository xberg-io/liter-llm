#!/usr/bin/env bats
#
# Contract tests for scripts/e2e/run-with-mock-server.sh.
#
# The mock-server binary is a stub that announces two URLs and then waits for SIGTERM. The
# script's contract is what the command it runs can see (the exported URLs), the exit status it
# returns, and that the server does not outlive it. ~keep

setup() {
	bats_load_library xberg-bats
	xberg_setup

	SOURCE="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)/scripts/e2e/run-with-mock-server.sh"
	REPO="$XBERG_WORK/repo"
	SCRIPT="$REPO/scripts/e2e/run-with-mock-server.sh"
	mkdir -p "$(dirname "$SCRIPT")" "$REPO/fixtures"
	cp "$SOURCE" "$SCRIPT"

	MOCK_BIN="$XBERG_WORK/mock-server"
	export MOCK_SERVER_BIN="$MOCK_BIN"
	write_mock_server
}

# A server that records its pid and argv, prints the announcement lines, and exits on SIGTERM.
write_mock_server() {
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		'printf "%s\n" "$$" >"$XBERG_WORK/server.pid"' \
		'printf "%s\n" "$*" >"$XBERG_WORK/server.args"' \
		'printf "MOCK_SERVER_URL=http://127.0.0.1:4010\n"' \
		'printf "MOCK_SERVERS={\"edge-case\":\"http://127.0.0.1:4011\"}\n"' \
		"trap 'exit 0' TERM" \
		'while :; do sleep 0.05; done' >"$MOCK_BIN"
	chmod +x "$MOCK_BIN"
}

@test "run-with-mock-server should_print_usage_when_no_command_is_given" {
	run bash "$SCRIPT"

	xberg_assert_status 2
	xberg_assert_output_contains "usage:"
}

@test "run-with-mock-server should_fail_when_the_mock_server_binary_is_missing" {
	export MOCK_SERVER_BIN="$XBERG_WORK/absent-mock-server"

	run bash "$SCRIPT" 'true'

	xberg_assert_status 1
	xberg_assert_output_contains "mock-server binary not found at $XBERG_WORK/absent-mock-server"
}

@test "run-with-mock-server should_export_the_announced_url_to_the_command" {
	run bash "$SCRIPT" 'printf "%s\n" "$MOCK_SERVER_URL"'

	xberg_assert_status 0
	xberg_assert_output "http://127.0.0.1:4010"
}

@test "run-with-mock-server should_export_each_per_fixture_url_with_an_uppercase_name" {
	run bash "$SCRIPT" 'printf "%s\n" "$MOCK_SERVER_EDGE_CASE"'

	xberg_assert_status 0
	xberg_assert_output "http://127.0.0.1:4011"
}

@test "run-with-mock-server should_start_the_server_on_the_fixtures_directory" {
	export FIXTURES_DIR="$XBERG_WORK/custom-fixtures"

	run bash "$SCRIPT" 'true'

	xberg_assert_status 0
	xberg_assert_file "$XBERG_WORK/server.args" "$XBERG_WORK/custom-fixtures"
}

@test "run-with-mock-server should_return_the_exit_status_of_the_command" {
	run bash "$SCRIPT" 'exit 7'

	xberg_assert_status 7
}

@test "run-with-mock-server should_stop_the_server_when_the_command_ends" {
	run bash "$SCRIPT" 'true'
	xberg_assert_status 0

	local pid attempt
	pid="$(cat "$XBERG_WORK/server.pid")"
	for attempt in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
		kill -0 "$pid" 2>/dev/null || return 0
		sleep 0.25
	done
	echo "mock server $pid is still running" >&2
	kill "$pid" 2>/dev/null || true
	return 1
}

@test "run-with-mock-server should_fail_when_the_server_announces_no_url" {
	printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$MOCK_BIN"
	# The wait loop sleeps 50 ms per attempt; a no-op sleep skips the 10 second wait. ~keep
	xberg_stub sleep 'exit 0'

	run bash "$SCRIPT" 'true'

	xberg_assert_status 1
	xberg_assert_output_contains "mock-server did not announce a URL within 10s"
}
