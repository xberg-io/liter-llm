#!/usr/bin/env bats
#
# Contract tests for scripts/ci/php/create-ci-php-ini.sh.
#
# The script derives the repository root from its own location, so each test copies it into a
# scratch tree with the same relative path. The real target/ directory is never read. ~keep

setup() {
	bats_load_library xberg-bats
	xberg_setup

	SOURCE="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)/scripts/ci/php/create-ci-php-ini.sh"
	REPO="$XBERG_WORK/repo"
	SCRIPT="$REPO/scripts/ci/php/create-ci-php-ini.sh"
	RELEASE_DIR="$REPO/target/release"
	OUTPUT="$XBERG_WORK/out"
	mkdir -p "$(dirname "$SCRIPT")" "$RELEASE_DIR" "$OUTPUT"
	cp "$SOURCE" "$SCRIPT"

	# The script picks the library name from $OSTYPE, which bash sets itself and a test cannot
	# override, so the expected name follows the host.
	case "$(uname -s)" in
	Darwin) EXTENSION="libliter_llm_php.dylib" ;;
	*) EXTENSION="libliter_llm_php.so" ;;
	esac
}

run_script() {
	run env OUTPUT_DIR="$OUTPUT" bash "$SCRIPT"
}

@test "create-ci-php-ini should_write_an_ini_that_loads_the_built_extension" {
	: >"$RELEASE_DIR/$EXTENSION"

	run_script

	xberg_assert_status 0
	xberg_assert_output_contains "INI file created: $OUTPUT/php-liter-llm.ini"
	run cat "$OUTPUT/php-liter-llm.ini"
	xberg_assert_output_contains "extension=\"$RELEASE_DIR/$EXTENSION\""
}

@test "create-ci-php-ini should_write_into_the_working_directory_when_no_output_dir_is_given" {
	: >"$RELEASE_DIR/$EXTENSION"
	cd "$OUTPUT"

	run bash "$SCRIPT"

	xberg_assert_status 0
	[ -f "$OUTPUT/php-liter-llm.ini" ]
}

@test "create-ci-php-ini should_fail_and_list_candidates_when_the_extension_is_missing" {
	: >"$RELEASE_DIR/libliter_llm_other.bin"

	run_script

	xberg_assert_status 1
	xberg_assert_output_contains "ERROR: Built extension not found at $RELEASE_DIR/$EXTENSION"
	xberg_assert_output_contains "libliter_llm_other.bin"
	xberg_assert_file_absent "$OUTPUT/php-liter-llm.ini"
}

@test "create-ci-php-ini should_fail_when_the_ini_file_cannot_be_written" {
	: >"$RELEASE_DIR/$EXTENSION"
	OUTPUT="$XBERG_WORK/missing-directory"

	run_script

	xberg_assert_status 1
}
