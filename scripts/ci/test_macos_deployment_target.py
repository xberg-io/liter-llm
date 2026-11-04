"""Every macOS binary the release ships targets the macOS minimum the root Package.swift declares.

The release workflows set `MACOSX_DEPLOYMENT_TARGET` for the Rust and C objects of every native artifact: the FFI
libraries, the language extensions, the CLI and the Python wheels, whose platform tag follows it. The shared Swift
prepare workflow derives the bundle's target from Package.swift itself.
"""

import re
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
RELEASE_FILES = (
    ".github/workflows/publish.yaml",
    ".github/workflows/python-wheels.yaml",
    "mise-tasks/languages.toml",
)


def macos_minimum() -> str:
    (major,) = re.findall(r"\.macOS\(\.v(\d+)\)", (ROOT / "Package.swift").read_text(encoding="utf-8"))
    return f"{major}.0"


def workflow(name: str) -> dict:
    return yaml.safe_load((WORKFLOWS / name).read_text(encoding="utf-8"))


def test_every_deployment_target_in_the_release_files_is_the_package_minimum() -> None:
    found = {
        path: re.findall(r"MACOSX_DEPLOYMENT_TARGET\W+([0-9][0-9.]*)", (ROOT / path).read_text(encoding="utf-8"))
        for path in RELEASE_FILES
    }
    assert {path: set(values) for path, values in found.items()} == {path: {macos_minimum()} for path in RELEASE_FILES}


def test_publish_sets_the_target_for_every_job() -> None:
    publish = workflow("publish.yaml")
    assert publish["env"]["MACOSX_DEPLOYMENT_TARGET"] == macos_minimum()
    overrides = {name for name, job in publish["jobs"].items() if "MACOSX_DEPLOYMENT_TARGET" in (job.get("env") or {})}
    assert overrides == set()


def test_swift_bundle_and_task_build_for_the_package_minimum() -> None:
    # The shared prepare workflow derives the bundle's target from Package.swift; an override here could only
    # restate it or fail the run.
    caller = workflow("prepare-swift-release.yaml")["jobs"]["swift"]["with"]
    assert "macos-deployment-target" not in caller
    with (ROOT / "mise-tasks/languages.toml").open("rb") as tasks:
        assert tomllib.load(tasks)["swift:build"]["env"]["MACOSX_DEPLOYMENT_TARGET"] == macos_minimum()


def test_macos_wheels_build_and_are_tagged_for_the_package_minimum() -> None:
    job = workflow("python-wheels.yaml")["jobs"]["build"]
    build = next(step for step in job["steps"] if "build-python-wheels" in step.get("uses", ""))
    assert f"MACOSX_DEPLOYMENT_TARGET={macos_minimum()}" in build["with"]["cibw-environment-macos"].split()
    tag = macos_minimum().replace(".", "_")
    legs = {leg["label"]: leg["wheel_platform"] for leg in job["strategy"]["matrix"]["include"]}
    assert {label: platform for label, platform in legs.items() if label.startswith("macos-")} == {
        "macos-arm64": f"macosx_{tag}_arm64",
        "macos-x86_64": f"macosx_{tag}_x86_64",
    }


def test_go_readme_states_the_package_minimum() -> None:
    for path in ("templates/readme/go.md", "packages/go/README.md"):
        text = (ROOT / path).read_text(encoding="utf-8")
        assert re.findall(r"deployment target of ([0-9.]+[0-9])", text) == [macos_minimum()], path
