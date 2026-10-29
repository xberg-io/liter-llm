#!/usr/bin/env bats
#
# Contract tests for scripts/publish/update-homebrew-formula.sh.
#
# The script rewrites a formula in a tap checkout. The tests build a small formula in a scratch
# tap, stub curl to serve a known tarball, and assert the bytes the script leaves on disk.
# python3 and shasum stay real: the rewrite itself is what these tests exercise. ~keep

setup() {
	bats_load_library xberg-bats
	xberg_setup
	command -v shasum >/dev/null || skip "shasum is required"

	ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$ROOT/scripts/publish/update-homebrew-formula.sh"

	TAP="$XBERG_WORK/tap"
	FORMULA="$TAP/Formula/liter-llm.rb"
	mkdir -p "$TAP/Formula"
	write_formula

	TARBALL="tarball bytes for the release"
	EXPECTED_SHA="$(printf '%s' "$TARBALL" | shasum -a 256 | awk '{print $1}')"
	xberg_stub_curl_body "$TARBALL"

	export TAG="v2.0.0" VERSION="2.0.0" TAP_DIR="$TAP"
	unset DRY_RUN
}

write_formula() {
	cat >"$FORMULA" <<'EOF'
class LiterLlm < Formula
  desc "Universal LLM API client"
  homepage "https://github.com/xberg-io/liter-llm"
  url "https://github.com/xberg-io/liter-llm/archive/v1.0.0.tar.gz"
  sha256 "1111111111111111111111111111111111111111111111111111111111111111"
  license "MIT"

  bottle do
    sha256 cellar: :any_skip_relocation, arm64_sonoma: "2222222222222222222222222222222222222222222222222222222222222222"
  end

  depends_on "rust" => :build
end
EOF
}

@test "update-homebrew-formula should_require_a_tag" {
	unset TAG

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "TAG is required"
}

@test "update-homebrew-formula should_require_a_version" {
	unset VERSION

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "VERSION is required"
}

@test "update-homebrew-formula should_require_a_tap_directory" {
	unset TAP_DIR

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "TAP_DIR is required"
}

@test "update-homebrew-formula should_fail_when_the_formula_is_missing" {
	export TAP_DIR="$XBERG_WORK/empty-tap"
	mkdir -p "$TAP_DIR"

	run bash "$SCRIPT"

	xberg_assert_status 1
	xberg_assert_output_contains "Missing $TAP_DIR/Formula/liter-llm.rb"
}

@test "update-homebrew-formula should_leave_the_formula_untouched_in_a_dry_run" {
	export DRY_RUN=true
	local before
	before="$(cat "$FORMULA")"
	xberg_stub_curl_offline

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_output_contains "[dry-run] would set url to: https://github.com/xberg-io/liter-llm/archive/v2.0.0.tar.gz"
	xberg_assert_file "$FORMULA" "$before"
}

@test "update-homebrew-formula should_set_the_source_url_and_checksum" {
	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_output_contains "sha256: $EXPECTED_SHA"
	run cat "$FORMULA"
	xberg_assert_output_contains 'url "https://github.com/xberg-io/liter-llm/archive/v2.0.0.tar.gz"'
	xberg_assert_output_contains "sha256 \"$EXPECTED_SHA\""
}

@test "update-homebrew-formula should_keep_the_bottle_block_unchanged" {
	run bash "$SCRIPT"

	xberg_assert_status 0
	run cat "$FORMULA"
	xberg_assert_output_contains 'arm64_sonoma: "2222222222222222222222222222222222222222222222222222222222222222"'
	case "$output" in
	*1.0.0.tar.gz* | *1111111111111111*)
		echo "the previous release url or checksum is still in the formula" >&2
		return 1
		;;
	esac
}

@test "update-homebrew-formula should_add_the_build_dependencies_once" {
	run bash "$SCRIPT"
	xberg_assert_status 0

	run grep -c "depends_on 'protobuf' => :build" "$FORMULA"
	xberg_assert_output 1
	run grep -c "depends_on 'openssl@3' => :build" "$FORMULA"
	xberg_assert_output 1
}

@test "update-homebrew-formula should_not_change_the_formula_on_a_second_run" {
	run bash "$SCRIPT"
	xberg_assert_status 0
	local after_first
	after_first="$(cat "$FORMULA")"

	run bash "$SCRIPT"

	xberg_assert_status 0
	xberg_assert_file "$FORMULA" "$after_first"
}

@test "update-homebrew-formula should_fail_and_keep_the_formula_when_the_url_anchor_is_gone" {
	printf '%s\n' 'class LiterLlm < Formula' '  depends_on "rust" => :build' 'end' >"$FORMULA"
	local before
	before="$(cat "$FORMULA")"

	run bash "$SCRIPT"

	[ "$status" -ne 0 ]
	xberg_assert_output_contains "no match for the source url"
	xberg_assert_file "$FORMULA" "$before"
}

@test "update-homebrew-formula should_fail_when_the_rust_anchor_for_the_build_dependencies_is_gone" {
	grep -v 'depends_on "rust"' "$FORMULA" >"$FORMULA.next"
	mv "$FORMULA.next" "$FORMULA"
	local before
	before="$(cat "$FORMULA")"

	run bash "$SCRIPT"

	[ "$status" -ne 0 ]
	xberg_assert_output_contains "no match for the \`depends_on 'rust'\` anchor"
	xberg_assert_file "$FORMULA" "$before"
}

@test "update-homebrew-formula should_fail_when_the_tarball_download_fails" {
	xberg_stub_exit curl 22
	local before
	before="$(cat "$FORMULA")"

	run bash "$SCRIPT"

	xberg_assert_status 22
	xberg_assert_file "$FORMULA" "$before"
}
