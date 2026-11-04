"""The Swift release path: the checksum is pinned before the tag, and publishing never moves a tag.

SwiftPM resolves `from:` and `exact:` versions to the core `vX.Y.Z` tag, so that tag's root `Package.swift` must
already carry the artifact bundle's checksum. The prepare workflow builds the bundle once, pins its checksum on main
and stages the exact ZIP on a draft release whose target is that commit; publishing the draft creates the tag. The
publish workflow only verifies and completes that state through `xberg-io/actions/publish-swift-release`.
"""

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
BUNDLE = "LiterLlm-rs.artifactbundle.zip"
RECEIPT = "dist/swift-artifactbundle/LiterLlm-rs.artifactbundle.release.json"


def workflow(name: str) -> dict:
    return yaml.safe_load((WORKFLOWS / name).read_text(encoding="utf-8"))


def publish_jobs() -> dict:
    return workflow("publish.yaml")["jobs"]


def prepare_job() -> dict:
    return workflow("prepare-swift-release.yaml")["jobs"]["prepare-swift-release"]


def scripts(job: dict) -> str:
    return "\n".join(step.get("run", "") for step in job.get("steps", []))


def uses(job: dict, action: str) -> list[dict]:
    return [step for step in job.get("steps", []) if step.get("uses", "").split("@")[0] == action]


def needs(job: dict) -> list[str]:
    value = job.get("needs", [])
    return [value] if isinstance(value, str) else list(value)


def commands(text: str) -> list[str]:
    return [line.strip() for line in text.splitlines() if line.strip() and not line.strip().startswith("#")]


def test_no_release_workflow_creates_or_moves_a_swift_tag() -> None:
    for name in ("publish.yaml", "prepare-swift-release.yaml"):
        lines = commands((WORKFLOWS / name).read_text(encoding="utf-8"))
        writes = [line for line in lines if "git ls-remote" not in line]
        tagging = [
            line for line in writes if re.search(r"\bgit tag\b|refs/tags/|\bgit push\b.*(--force|\s-f\b|\s\+)", line)
        ]
        assert [line for line in tagging if "MODULE_TAG" not in line] == [], name
        assert not [line for line in lines if "swift-${VERSION}" in line or "SWIFT_TAG" in line], name


def test_publish_never_rebuilds_or_rewrites_the_swift_manifest() -> None:
    jobs = publish_jobs()
    assert "swift-artifactbundle" not in jobs
    for name, job in jobs.items():
        assert not uses(job, "xberg-io/actions/build-swift-artifactbundle"), name
        text = scripts(job)
        assert "__ALEF_SWIFT_CHECKSUM__" not in text, name
        assert not re.search(r"gh release upload[^\n]*Package\.swift", text), name


def test_publish_verifies_the_tag_through_the_shared_action() -> None:
    job = publish_jobs()["upload-swift-release"]
    assert needs(job) == ["prepare", "validate-versions"]
    assert "needs.prepare.outputs.is_tag == 'true'" in job["if"]
    assert "needs.prepare.outputs.release_swift == 'true'" in job["if"]
    assert "needs.validate-versions.result == 'success'" in job["if"]
    (checkout,) = uses(job, "actions/checkout")
    assert checkout["with"] == {
        "ref": "${{ needs.prepare.outputs.tag }}",
        "fetch-depth": 0,
        "token": "${{ steps.app-token.outputs.token }}",
        "persist-credentials": True,
    }
    (publish,) = uses(job, "xberg-io/actions/publish-swift-release")
    assert publish["uses"] == "xberg-io/actions/publish-swift-release@v1"
    assert publish["with"] == {
        "tag": "${{ needs.prepare.outputs.tag }}",
        "bundle-zip": "",
        "receipt": RECEIPT,
        "dry-run": "${{ needs.prepare.outputs.dry_run }}",
        "token": "${{ steps.app-token.outputs.token }}",
    }
    steps = job["steps"]
    download = next(step for step in steps if "gh release download" in step.get("run", ""))
    assert steps.index(download) < steps.index(publish)
    assert "LiterLlm-rs.artifactbundle.release.json" in download["run"]


def test_release_notes_point_swift_users_at_the_core_version_tag() -> None:
    job = publish_jobs()["upload-swift-release"]
    step = next(s for s in job["steps"] if s.get("name") == "Update release notes with Swift install instructions")
    assert step["env"]["CHECKSUM"] == "${{ steps.publish.outputs.checksum }}"
    script = step["run"]
    assert '.package(url: "https://github.com/xberg-io/liter-llm", from: "${VERSION}")' in script
    assert 'branch: "release/swift/${VERSION}"' in script
    assert "swift-${VERSION}" not in script
    assert "placeholder" not in script


def test_jobs_gated_on_the_swift_job_still_reference_it() -> None:
    jobs = publish_jobs()
    for name in ("publish-zig", "promote-release", "release-finalize"):
        assert "upload-swift-release" in needs(jobs[name]), name
    for name, job in jobs.items():
        assert set(needs(job)) <= set(jobs), name


def test_prepare_pins_the_checksum_from_one_build_before_the_tag() -> None:
    document = workflow("prepare-swift-release.yaml")
    assert set(document[True]) == {"workflow_dispatch"}
    job = prepare_job()
    assert job["runs-on"] == "macos-26"
    (build,) = uses(job, "xberg-io/actions/build-swift-artifactbundle")
    assert build["with"]["artifact-name"] == "LiterLlm-rs"
    assert build["with"]["output-dir"] == "dist/swift-artifactbundle"
    assert build["with"]["receipt-version"] == "${{ steps.version.outputs.version }}"
    assert "dry-run" not in build["with"]
    text = scripts(job)
    lines = commands(text)
    assert "alef sync-versions --regen" in lines
    assert "git add Package.swift" in lines
    assert not [line for line in lines if line.startswith("git add") and line != "git add Package.swift"]
    assert 'git push origin "HEAD:refs/heads/main"' in lines
    create = next(line for line in lines if line.startswith("gh release create"))
    assert "--draft" in create
    assert '--target "${PINNED}"' in create
    for asset in (BUNDLE, "LiterLlm-rs.artifactbundle.release.json", f"{BUNDLE}.checksum"):
        assert asset in text
    assert text.index("alef sync-versions --regen") < text.index("git push") < text.index("gh release create")


def test_prepare_refuses_an_existing_tag_and_a_mismatched_pin() -> None:
    text = scripts(prepare_job())
    assert 'git ls-remote --exit-code --tags origin "refs/tags/v${VERSION}"' in text
    assert "shasum -a 256" in text
    assert "__ALEF_" in text


def test_dry_run_pushes_and_releases_nothing() -> None:
    job = prepare_job()
    writers = [
        step for step in job["steps"] if re.search(r"\bgit push\b|gh release (create|upload)", step.get("run", ""))
    ]
    assert writers
    for step in writers:
        assert step["if"] == "${{ !inputs.dry_run }}", step.get("name")
