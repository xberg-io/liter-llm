"""Verify the release archives satisfy the generated Go setup contract."""

import ast
import hashlib
import subprocess
from pathlib import Path

import pytest
import yaml
from check_native_library_size import validate_library_size
from prepare_go_release_assets import prepare

ROOT = Path(__file__).resolve().parents[2]
PUBLISH_WORKFLOW = ROOT / ".github" / "workflows" / "publish.yaml"


def test_alias_preserves_archive_and_writes_installer_checksum(tmp_path: Path) -> None:
    """The installer alias must contain exactly the published native payload."""
    source = tmp_path / "liter-llm-go-v2.1.0-rc.1-macos-arm64.tar.gz"
    payload = b"native archive bytes"
    source.write_bytes(payload)
    prepare(tmp_path, "liter-llm", "2.1.0-rc.1")
    alias = tmp_path / "liter-llm-go-macos-arm64.tar.gz"
    assert source.read_bytes() == alias.read_bytes() == payload
    assert alias.with_name(alias.name + ".sha256").read_text() == (
        f"{hashlib.sha256(payload).hexdigest()}  {alias.name}\n"
    )
    prepare(tmp_path, "liter-llm", "2.1.0-rc.1")
    assert alias.read_bytes() == payload


def test_missing_current_release_archives_fails(tmp_path: Path) -> None:
    """An empty or wrong-version artifact directory cannot pass packaging."""
    (tmp_path / "liter-llm-go-v1.0.0-macos-arm64.tar.gz").write_bytes(b"old")
    with pytest.raises(ValueError, match="No Go release archives"):
        prepare(tmp_path, "liter-llm", "2.0.0")


def test_conflicting_existing_alias_is_not_overwritten(tmp_path: Path) -> None:
    """Recovery must not replace a different already-prepared native payload."""
    (tmp_path / "liter-llm-go-v2.0.0-linux-x86_64.tar.gz").write_bytes(b"new")
    alias = tmp_path / "liter-llm-go-linux-x86_64.tar.gz"
    alias.write_bytes(b"existing")
    with pytest.raises(ValueError, match="differs"):
        prepare(tmp_path, "liter-llm", "2.0.0")
    assert alias.read_bytes() == b"existing"


def test_publish_workflow_builds_both_musl_go_assets() -> None:
    workflow = PUBLISH_WORKFLOW.read_text()
    assert workflow.count("target: x86_64-unknown-linux-musl") == 1
    assert workflow.count("target: aarch64-unknown-linux-musl") == 1
    assert "needs.build-go-musl.result == 'success'" in workflow
    assert '*-linux-x86_64-musl)  lib_dir="linux-x86_64-musl"' in workflow
    assert '*-linux-aarch64-musl) lib_dir="linux-aarch64-musl"' in workflow


def test_go_archives_are_stripped_before_packaging() -> None:
    workflow = PUBLISH_WORKFLOW.read_text()
    assert workflow.count("Strip static archive debug symbols") == 2
    assert workflow.count('archive="target/${TARGET}/release/libliter_llm_ffi.a"') == 2
    assert workflow.index("Strip static archive debug symbols") < workflow.index("Package Go FFI")
    assert "if: runner.os == 'Linux'" in workflow
    assert 'strip -S "$archive"' not in workflow


def test_go_shared_libraries_strip_debug_info_and_enforce_size_budget() -> None:
    workflow = PUBLISH_WORKFLOW.read_text()
    assert "CARGO_PROFILE_RELEASE_STRIP: debuginfo" in workflow
    assert "Verify shared Go FFI library size budget" in workflow
    assert 'check_native_library_size.py "$SHARED_LIBRARY" --max-mib 40' in workflow
    assert workflow.index("CARGO_PROFILE_RELEASE_STRIP: debuginfo") < workflow.index("Package Go FFI")


def test_native_library_size_budget_checks_exactly_one_real_file(tmp_path: Path) -> None:
    library = tmp_path / "libliter_llm_ffi.dylib"
    library.write_bytes(b"x" * 32)
    assert validate_library_size(library, max_bytes=32) == 32

    library.write_bytes(b"x" * 33)
    with pytest.raises(ValueError, match="exceeds the 32-byte budget"):
        validate_library_size(library, max_bytes=32)

    with pytest.raises(FileNotFoundError):
        validate_library_size(tmp_path / "missing.dylib", max_bytes=32)


def test_tagged_release_uploads_catalog_checksum() -> None:
    workflow = PUBLISH_WORKFLOW.read_text()
    assert "GH_REPO: ${{ github.repository }}" in workflow
    assert 'CATALOG_DIR="${RUNNER_TEMP:?}"' in workflow
    assert "sha256sum catalog.json > catalog.json.sha256" in workflow
    assert (
        'gh release upload "$TAG" "$CATALOG_DIR/catalog.json" "$CATALOG_DIR/catalog.json.sha256" --clobber' in workflow
    )


def evaluate_workflow_expression(expression: str, values: dict[str, object]) -> object:
    """Evaluate the workflow's boolean/equality subset without executing expressions."""
    translated = " ".join(expression.removeprefix("${{").removesuffix("}}").split())
    translated = translated.replace("&&", "and").replace("||", "or")

    def evaluate(node: ast.expr) -> object:
        if isinstance(node, ast.BoolOp):
            result = evaluate(node.values[0])
            for value in node.values[1:]:
                if isinstance(node.op, ast.And) and not result:
                    return result
                if isinstance(node.op, ast.Or) and result:
                    return result
                result = evaluate(value)
            return result
        if isinstance(node, ast.Compare):
            assert len(node.ops) == len(node.comparators) == 1
            left, right = evaluate(node.left), evaluate(node.comparators[0])
            if isinstance(node.ops[0], ast.Eq):
                return left == right
            assert isinstance(node.ops[0], ast.NotEq)
            return left != right
        if isinstance(node, ast.Constant):
            return node.value
        if isinstance(node, ast.Name) and node.id in ("true", "false"):
            return node.id == "true"
        if isinstance(node, ast.Attribute):
            return values[ast.unparse(node)]
        raise AssertionError(f"unsupported workflow expression: {ast.dump(node)}")

    return evaluate(ast.parse(translated, mode="eval").body)


@pytest.mark.parametrize(
    "destructive",
    [
        (False, False, False, False),
        (True, False, False, False),
        (False, True, False, False),
        (False, False, True, False),
        (False, False, False, True),
    ],
)
@pytest.mark.parametrize(
    "dry_run",
    [
        (False, False),
        (None, None),
        ("false", "false"),
        (True, False),
        ("true", False),
        (False, True),
        (False, "true"),
        (True, True),
        ("true", "true"),
    ],
)
def test_real_runs_reject_tag_moving_options(
    destructive: tuple[bool, bool, bool, bool], dry_run: tuple[object, object]
) -> None:
    """Release tags are immutable: both dispatch channels reject republish and force_republish unless dry."""
    workflow = yaml.safe_load(PUBLISH_WORKFLOW.read_text())
    steps = workflow["jobs"]["prepare"]["steps"]
    assert not [step for step in steps if "retag-for-republish" in step.get("uses", "")]
    (reject,) = [step for step in steps if step.get("name") == "Reject destructive release options"]

    def text(value: object) -> str:
        return "" if value is None else str(value).lower()

    env = dict(
        zip(
            ("REPUBLISH", "FORCE_REPUBLISH", "PAYLOAD_REPUBLISH", "PAYLOAD_FORCE_REPUBLISH"),
            map(text, destructive),
            strict=True,
        )
    )
    env |= {"DRY_RUN": text(dry_run[0]), "PAYLOAD_DRY_RUN": text(dry_run[1])}
    result = subprocess.run(["bash", "-c", reject["run"]], env=env, capture_output=True, text=True, check=False)
    rejected = any(destructive) and not any(value is True or value == "true" for value in dry_run)
    assert result.returncode == (1 if rejected else 0), result.stderr


@pytest.mark.parametrize("dry_run", ["true", "false"])
@pytest.mark.parametrize("requested_ref", [None, "", "main", "a" * 40])
@pytest.mark.parametrize("event_ref", ["refs/heads/main", "refs/tags/v2.2.5"])
def test_release_checkout_forwards_resolved_metadata_ref(
    dry_run: str, requested_ref: str | None, event_ref: str
) -> None:
    """A dry-run checkout must retain the metadata action's immutable SHA."""
    workflow = yaml.safe_load(PUBLISH_WORKFLOW.read_text())
    expression = workflow["jobs"]["prepare"]["outputs"]["checkout_ref"]
    resolved_ref = "b" * 40 if dry_run == "true" else "refs/tags/v2.2.5"
    values = {
        "steps.meta.outputs.dry_run": dry_run,
        "steps.meta.outputs.checkout_ref": resolved_ref,
        "inputs.ref": requested_ref,
        "github.ref": event_ref,
    }
    assert evaluate_workflow_expression(expression, values) == resolved_ref


def test_checkout_regression_rejects_the_retired_moving_branch_expression() -> None:
    """The negative control must fail for the original semantic defect."""
    retired = (
        "${{ steps.meta.outputs.dry_run == 'true' && (inputs.ref || github.ref) || steps.meta.outputs.checkout_ref }}"
    )
    values = {
        "steps.meta.outputs.dry_run": "true",
        "steps.meta.outputs.checkout_ref": "b" * 40,
        "inputs.ref": "",
        "github.ref": "refs/heads/main",
    }
    assert evaluate_workflow_expression(retired, values) == "refs/heads/main"
    with pytest.raises(AssertionError):
        assert evaluate_workflow_expression(retired, values) == "b" * 40


@pytest.mark.parametrize("republish", [False, True])
@pytest.mark.parametrize("dry_run", [True, "true", False, "false", None])
@pytest.mark.parametrize("payload_dry_run", [True, "true", False, "false", None])
@pytest.mark.parametrize("requested_ref", [None, "", "release-candidate", "a" * 40])
@pytest.mark.parametrize("event_ref", ["refs/heads/main", "refs/tags/v2.2.5"])
def test_initial_metadata_checkout_pins_dry_runs_and_preserves_ref_priority(
    republish: bool, dry_run: object, payload_dry_run: object, requested_ref: str | None, event_ref: str
) -> None:
    """Metadata and build checkout must read the same dispatch source."""
    workflow = yaml.safe_load(PUBLISH_WORKFLOW.read_text())
    checkouts = [step for step in workflow["jobs"]["prepare"]["steps"] if step.get("name") == "Checkout"]
    assert len(checkouts) == 1
    values = {
        "inputs.republish": republish,
        "inputs.ref": requested_ref,
        "inputs.dry_run": dry_run,
        "github.event.client_payload.dry_run": payload_dry_run,
        "github.event.repository.default_branch": "main",
        "github.sha": "b" * 40,
        "github.ref": event_ref,
    }
    if republish:
        expected = requested_ref or "main"
    elif requested_ref:
        expected = requested_ref
    elif dry_run is True or dry_run == "true" or payload_dry_run is True or payload_dry_run == "true":
        expected = "b" * 40
    else:
        expected = event_ref
    assert evaluate_workflow_expression(checkouts[0]["with"]["ref"], values) == expected


def test_initial_checkout_negative_control_detects_the_moving_branch() -> None:
    """The retired initial checkout must fail the immutable-source contract."""
    retired = (
        "${{ (inputs.republish == true && (inputs.ref || github.event.repository.default_branch)) "
        "|| inputs.ref || github.ref }}"
    )
    values = {
        "inputs.republish": False,
        "inputs.ref": "",
        "inputs.dry_run": True,
        "github.event.repository.default_branch": "main",
        "github.sha": "b" * 40,
        "github.ref": "refs/heads/main",
    }
    assert evaluate_workflow_expression(retired, values) == "refs/heads/main"
    with pytest.raises(AssertionError):
        assert evaluate_workflow_expression(retired, values) == values["github.sha"]
