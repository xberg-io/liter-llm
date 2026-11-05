"""Every macOS binary the release ships targets the lowest macOS that the code and the toolchain need.

That is macOS 10.13 on x86_64 and macOS 11.0 on arm64: Apple Silicon has no macOS before 11, and clang, rustc and
maturin raise an arm64 target below 11 to 11.0. The `MACOSX_DEPLOYMENT_TARGET` of publish.yaml is the one source of
truth. The other release files repeat it for the Rust and C objects of every native artifact: the FFI libraries,
the language extensions, the CLI, the Swift build task and the Python wheels, whose platform tag follows it.
The Swift package has its own minimum, which test_swift_macos_minimum.py guards; the shared Swift prepare workflow
derives the release bundle's target from Package.swift itself.
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


ARM64_MINIMUM = "11.0"


def workflow(name: str) -> dict:
    return yaml.safe_load((WORKFLOWS / name).read_text(encoding="utf-8"))


def deployment_target() -> str:
    return workflow("publish.yaml")["env"]["MACOSX_DEPLOYMENT_TARGET"]


def wheel_tag(version: str, arch: str) -> str:
    return f"macosx_{version.replace('.', '_')}_{arch}"


def test_the_deployment_target_is_10_13_and_below_the_arm64_minimum() -> None:
    assert deployment_target() == "10.13"
    assert tuple(map(int, deployment_target().split("."))) < tuple(map(int, ARM64_MINIMUM.split(".")))


def test_every_deployment_target_in_the_release_files_is_the_publish_target() -> None:
    found = {
        path: re.findall(r"MACOSX_DEPLOYMENT_TARGET\W+([0-9][0-9.]*)", (ROOT / path).read_text(encoding="utf-8"))
        for path in RELEASE_FILES
    }
    assert {path: set(values) for path, values in found.items()} == {
        path: {deployment_target()} for path in RELEASE_FILES
    }


def test_publish_sets_the_target_for_every_job() -> None:
    publish = workflow("publish.yaml")
    overrides = {name for name, job in publish["jobs"].items() if "MACOSX_DEPLOYMENT_TARGET" in (job.get("env") or {})}
    assert overrides == set()


def test_swift_task_builds_for_the_publish_target() -> None:
    # The shared prepare workflow derives the release bundle's target from Package.swift; an override here could
    # only restate it or fail the run.
    caller = workflow("prepare-swift-release.yaml")["jobs"]["swift"]["with"]
    assert "macos-deployment-target" not in caller
    with (ROOT / "mise-tasks/languages.toml").open("rb") as tasks:
        assert tomllib.load(tasks)["swift:build"]["env"]["MACOSX_DEPLOYMENT_TARGET"] == deployment_target()


def test_macos_wheels_build_for_the_publish_target_and_arm64_is_tagged_11_0() -> None:
    job = workflow("python-wheels.yaml")["jobs"]["build"]
    build = next(step for step in job["steps"] if "build-python-wheels" in step.get("uses", ""))
    assert f"MACOSX_DEPLOYMENT_TARGET={deployment_target()}" in build["with"]["cibw-environment-macos"].split()
    legs = {leg["label"]: leg["wheel_platform"] for leg in job["strategy"]["matrix"]["include"]}
    assert {label: platform for label, platform in legs.items() if label.startswith("macos-")} == {
        "macos-arm64": wheel_tag(ARM64_MINIMUM, "arm64"),
        "macos-x86_64": wheel_tag(deployment_target(), "x86_64"),
    }


def test_go_readme_states_the_publish_target() -> None:
    for path in ("templates/readme/go.md", "packages/go/README.md"):
        text = (ROOT / path).read_text(encoding="utf-8")
        assert re.findall(r"deployment target of ([0-9.]+[0-9])", text) == [deployment_target()], path
