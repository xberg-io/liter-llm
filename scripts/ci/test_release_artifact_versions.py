"""Exercise actual release workflow commands against canonical package versions."""

import copy
import os
import subprocess
from pathlib import Path

import pytest
import tomllib
import yaml
from packaging.version import Version
from prepare_go_release_assets import prepare

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/publish.yaml"
SHA = "3fa1947f35968d15eeef7e3056c7f3c7563e4a14"


@pytest.fixture
def canonical_version() -> str:
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        return tomllib.load(manifest)["workspace"]["package"]["version"]


def workflow_jobs() -> dict:
    return yaml.safe_load(WORKFLOW.read_text())["jobs"]


def version_forms(tmp_path: Path, version: str, **overrides: str) -> dict[str, str]:
    values = {
        "steps.meta.outputs.version": version,
        "steps.meta.outputs.tag": f"v{version}",
        "steps.meta.outputs.dry_run": "true",
        "steps.meta.outputs.is_tag": "true",
        "steps.meta.outputs.target_sha": SHA,
        "inputs.tag || github.event.client_payload.tag": "",
    }
    values.update(overrides)
    steps = [step for step in workflow_jobs()["prepare"]["steps"] if step.get("id") == "version_forms"]
    assert len(steps) == 1
    step = steps[0]
    output = tmp_path / "version-outputs"
    environment = os.environ.copy()
    environment["GITHUB_OUTPUT"] = str(output)
    for key, expression in step["env"].items():
        assert expression.startswith("${{ ")
        assert expression.endswith(" }}")
        environment[key] = values[expression[4:-3]]
    result = subprocess.run(
        ["bash", "-euo", "pipefail", "-c", step["run"]],
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    rows = output.read_text().splitlines()
    pairs = [row.split("=", 1) for row in rows]
    assert all(len(pair) == 2 for pair in pairs)
    assert len(pairs) == len({pair[0] for pair in pairs})
    return dict(pairs)


def current_dry_run_forms(tmp_path: Path, canonical: str) -> tuple[str, dict[str, str]]:
    version = f"{canonical}-dryrun-{SHA[:7]}"
    forms = version_forms(tmp_path, version)
    assert forms["artifact_version"] == canonical
    assert forms["pypi_version"] == str(Version(canonical))
    return version, forms


def run_hermes_guard(tmp_path: Path, expected: str, actual: str) -> subprocess.CompletedProcess:
    steps = workflow_jobs()["publish-plugin-hermes"]["steps"]
    guards = [step for step in steps if step.get("name") == "Assert the plugin bundle matches the release"]
    assert len(guards) == 1
    script = guards[0]["run"].replace("${{ needs.prepare.outputs.pypi_version }}", expected)
    assert "${{" not in script
    manifest = tmp_path / "plugin/.hermes/package/pyproject.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(f'[project]\nname = "synthetic-hermes"\nversion = "{actual}"\n')
    return subprocess.run(
        ["bash", "-euo", "pipefail", "-c", script],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )


def go_expected_version(forms: dict[str, str], version: str) -> str:
    steps = workflow_jobs()["upload-go-ffi"]["steps"]
    alias_steps = [step for step in steps if step.get("name") == "Prepare Go installer archive aliases and checksums"]
    assert len(alias_steps) == 1
    expression = alias_steps[0]["env"]["RELEASE_VERSION"]
    outputs = {"version": version, **forms}
    prefix = "${{ needs.prepare.outputs."
    assert expression.startswith(prefix)
    assert expression.endswith(" }}")
    return outputs[expression[len(prefix) : -3]]


def test_synthesized_dry_run_validates_current_hermes_manifest(tmp_path: Path, canonical_version: str) -> None:
    _, forms = current_dry_run_forms(tmp_path, canonical_version)
    expected = str(Version(canonical_version))
    result = run_hermes_guard(tmp_path, forms["pypi_version"], expected)
    assert result.returncode == 0, result.stdout + result.stderr
    assert f"matches the release at {expected}" in result.stdout


def test_synthesized_dry_run_matches_actual_go_archive_name(tmp_path: Path, canonical_version: str) -> None:
    version, forms = current_dry_run_forms(tmp_path, canonical_version)
    assert go_expected_version(forms, version) == canonical_version
    archive = tmp_path / f"liter-llm-go-v{canonical_version}-linux-x86_64-musl.tar.gz"
    archive.write_bytes(b"synthetic opaque native payload")
    prepare(tmp_path, "liter-llm", go_expected_version(forms, version))
    alias = tmp_path / "liter-llm-go-linux-x86_64-musl.tar.gz"
    assert alias.read_bytes() == archive.read_bytes()
    assert alias.with_name(alias.name + ".sha256").is_file()


@pytest.mark.parametrize(
    ("version", "pypi", "gem"),
    [
        ("2.2.5", "2.2.5", "2.2.5"),
        ("2.2.5-rc.1", "2.2.5rc1", "2.2.5.pre.rc.1"),
        ("2.2.5-alpha.2", "2.2.5a2", "2.2.5.pre.alpha.2"),
        ("2.2.5-beta.3", "2.2.5b3", "2.2.5.pre.beta.3"),
    ],
)
def test_real_tag_versions_preserve_ecosystem_spelling(
    tmp_path: Path,
    version: str,
    pypi: str,
    gem: str,
) -> None:
    forms = version_forms(tmp_path, version, **{"steps.meta.outputs.dry_run": "false"})
    assert forms == {"artifact_version": version, "pypi_version": pypi, "gem_version": gem}


@pytest.mark.parametrize(
    "overrides",
    [
        {"steps.meta.outputs.dry_run": "false"},
        {"steps.meta.outputs.is_tag": "false"},
        {"inputs.tag || github.event.client_payload.tag": "v{canonical}-dryrun-{sha}"},
        {"steps.meta.outputs.target_sha": "a" * 40},
        {"steps.meta.outputs.tag": "v{canonical}-custom"},
    ],
)
def test_arbitrary_ref_or_explicit_tag_is_not_normalized(
    tmp_path: Path, canonical_version: str, overrides: dict[str, str]
) -> None:
    baseline = tmp_path / "baseline"
    baseline.mkdir()
    version, _ = current_dry_run_forms(baseline, canonical_version)
    current_overrides = {
        key: value.format(canonical=canonical_version, sha=SHA[:7]) for key, value in overrides.items()
    }
    forms = version_forms(tmp_path, version, **current_overrides)
    assert forms["artifact_version"] == version


def test_stale_hermes_manifest_still_fails_synthesized_dry_run(tmp_path: Path, canonical_version: str) -> None:
    _, forms = current_dry_run_forms(tmp_path, canonical_version)
    expected = str(Version(canonical_version))
    stale = "0.0.0" if expected != "0.0.0" else "0.0.1"
    assert stale != expected
    result = run_hermes_guard(tmp_path, forms["pypi_version"], stale)
    assert result.returncode == 1
    assert f"declares {stale} but the release is {expected}" in result.stdout


def test_wrong_go_archive_version_still_fails_synthesized_dry_run(tmp_path: Path, canonical_version: str) -> None:
    version, forms = current_dry_run_forms(tmp_path, canonical_version)
    assert go_expected_version(forms, version) == canonical_version
    stale = "0.0.0" if canonical_version != "0.0.0" else "0.0.1"
    assert stale != canonical_version
    (tmp_path / f"liter-llm-go-v{stale}-linux-x86_64-musl.tar.gz").write_bytes(b"old opaque payload")
    with pytest.raises(ValueError, match="No Go release archives"):
        prepare(tmp_path, "liter-llm", go_expected_version(forms, version))
    assert not (tmp_path / "liter-llm-go-linux-x86_64-musl.tar.gz").exists()


def test_synthesized_prerelease_removes_only_the_virtual_suffix() -> None:
    from release_artifact_versions import release_versions

    forms = release_versions(
        "2.2.5-rc.1-dryrun-3fa1947",
        "2.2.5-rc.1",
        tag="v2.2.5-rc.1-dryrun-3fa1947",
        dry_run="true",
        is_tag="true",
        requested_tag="",
        target_sha=SHA,
    )
    assert forms == {
        "artifact_version": "2.2.5-rc.1",
        "pypi_version": "2.2.5rc1",
        "gem_version": "2.2.5.pre.rc.1",
    }


def test_canonical_mismatch_does_not_normalize_an_unrelated_version() -> None:
    from release_artifact_versions import release_versions

    forms = release_versions(
        "9.9.9-dryrun-3fa1947",
        "2.2.5",
        tag="v9.9.9-dryrun-3fa1947",
        dry_run="true",
        is_tag="true",
        requested_tag="",
        target_sha=SHA,
    )
    assert forms["artifact_version"] == "9.9.9-dryrun-3fa1947"


def test_workflow_and_ci_run_the_real_tasks_and_regression_file() -> None:
    jobs = workflow_jobs()
    forms = [step for step in jobs["prepare"]["steps"] if step.get("id") == "version_forms"]
    assert len(forms) == 1
    assert forms[0]["run"] == "task ci:release-version-forms"
    aliases = [
        step
        for step in jobs["upload-go-ffi"]["steps"]
        if step.get("name") == "Prepare Go installer archive aliases and checksums"
    ]
    assert len(aliases) == 1
    assert aliases[0]["run"] == "task ci:prepare-go-release-assets"
    tasks = yaml.safe_load((ROOT / "Taskfile.yml").read_text())["tasks"]
    assert tasks["ci:release-version-forms"]["cmds"] == ["python3 scripts/ci/release_artifact_versions.py"]
    assert tasks["ci:prepare-go-release-assets"]["cmds"] == [
        'python3 scripts/ci/prepare_go_release_assets.py go-ffi-release liter-llm "$RELEASE_VERSION"'
    ]
    regression_commands = "\n".join(tasks["test:release-regressions"]["cmds"])
    assert regression_commands.count("scripts/ci/test_release_artifact_versions.py") == 1


TASK_ACTION = "xberg-io/actions/install-task@fdddc35399a9eb87cb9c35ae87346ef38e7bbc29"
RELEASE_TASKS = {"task ci:release-version-forms", "task ci:prepare-go-release-assets"}


def assert_release_task_prerequisites(jobs: dict) -> None:
    consumers = {
        job for job, config in jobs.items() if any(step.get("run") in RELEASE_TASKS for step in config.get("steps", []))
    }
    assert consumers == {"prepare", "upload-go-ffi"}
    for job in sorted(consumers):
        steps = jobs[job]["steps"]
        installers = [
            (index, step)
            for index, step in enumerate(steps)
            if step.get("uses", "").startswith("xberg-io/actions/install-task@")
        ]
        assert len(installers) == 1, f"Task installer count for {job}"
        index, installer = installers[0]
        assert installer["uses"] == TASK_ACTION, f"Task installer pin for {job}"
        first_task = min(index for index, step in enumerate(steps) if step.get("run") in RELEASE_TASKS)
        assert index < first_task, f"Task installer order for {job}"


def test_release_jobs_install_pinned_task_before_consuming_it() -> None:
    assert_release_task_prerequisites(workflow_jobs())


@pytest.mark.parametrize("job", ["prepare", "upload-go-ffi"])
@pytest.mark.parametrize("mutation", ["missing", "after-task", "mutable-pin"])
def test_task_prerequisite_guard_rejects_missing_late_or_mutable_installers(job: str, mutation: str) -> None:
    jobs = copy.deepcopy(workflow_jobs())
    steps = jobs[job]["steps"]
    installers = [step for step in steps if step.get("uses") == TASK_ACTION]
    assert len(installers) == 1
    installer = installers[0]
    if mutation == "missing":
        steps.remove(installer)
        expected = f"Task installer count for {job}"
    elif mutation == "after-task":
        steps.remove(installer)
        steps.append(installer)
        expected = f"Task installer order for {job}"
    else:
        installer["uses"] = "xberg-io/actions/install-task@v1"
        expected = f"Task installer pin for {job}"
    with pytest.raises(AssertionError, match=expected):
        assert_release_task_prerequisites(jobs)
