"""The Swift release path: the checksum is pinned before the tag, and publishing never moves a tag.

SwiftPM resolves `from:` and `exact:` versions to the core `vX.Y.Z` tag, so that tag's root `Package.swift` must
already carry the artifact bundle's checksum. Prepare Swift release calls the shared xberg-io/actions workflow, which
builds the bundle once, pins its checksum, pushes that commit with the tag and stages the exact ZIP and receipt on a
draft release. The publish workflow only verifies and completes that state through
`xberg-io/actions/publish-swift-release`.
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
        assert not [line for line in lines if "retag-for-republish" in line], name


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


PREPARE_OUTPUTS = "needs.prepare.outputs"
PREPARE_OK = "needs.prepare.result == 'success'"
VALIDATED = "needs.validate-versions.result == 'success'"
SEED = (
    "${{ steps.meta.outputs.is_tag == 'true' && steps.meta.outputs.release_swift == 'true' && "
    "steps.meta.outputs.dry_run != 'true' }}"
)


def ancestors(jobs: dict, name: str) -> set[str]:
    seen: set[str] = set()
    pending = needs(jobs[name])
    while pending:
        parent = pending.pop()
        if parent not in seen:
            seen.add(parent)
            pending.extend(needs(jobs[parent]))
    return seen


def test_dry_run_never_touches_the_swift_tag() -> None:
    """A dry run's tag is a synthesized `<version>-dryrun-<sha>` that does not exist."""
    job = publish_jobs()["upload-swift-release"]
    assert f"{PREPARE_OUTPUTS}.dry_run != 'true'" in job["if"]
    assert "dry_run" not in yaml.safe_dump(job["steps"])
    (seed,) = uses(publish_jobs()["prepare"], "xberg-io/actions/publish-swift-release")
    assert seed["if"] == SEED


def test_prepare_verifies_the_swift_seed_before_the_release_is_touched() -> None:
    job = publish_jobs()["prepare"]
    steps = job["steps"]
    (checkout,) = uses(job, "actions/checkout")
    assert checkout["with"]["fetch-depth"] == 0
    (seed,) = uses(job, "xberg-io/actions/publish-swift-release")
    assert seed["uses"] == "xberg-io/actions/publish-swift-release@v1"
    assert seed["with"] == {
        "tag": "${{ steps.meta.outputs.tag }}",
        "bundle-zip": f"dist/swift-artifactbundle/{BUNDLE}",
        "receipt": RECEIPT,
        "dry-run": "true",
        "token": "${{ steps.app-token.outputs.token }}",
    }
    download = next(step for step in steps if step.get("id") == "swift-seed")
    assert download["if"] == SEED
    assert BUNDLE in download["run"]
    assert "LiterLlm-rs.artifactbundle.release.json" in download["run"]
    for output in ("release_id", "tag_object", "swift_bundle_sha256", "swift_receipt_sha256"):
        assert job["outputs"][output] == f"${{{{ steps.swift-seed.outputs.{output} }}}}"
    ensure = next(step for step in steps if step.get("name") == "Ensure GitHub Release exists for tag")
    catalog = next(step for step in steps if step.get("name") == "Attach model catalog snapshot to release")
    assert steps.index(download) < steps.index(seed) < steps.index(ensure) < steps.index(catalog)


def test_every_publisher_is_gated_on_prepare() -> None:
    """A failed Swift seed fails prepare, which must stop every job that writes a registry or the release."""
    jobs = publish_jobs()
    writers = [
        name for name in jobs if re.match(r"(publish|upload|trigger|promote|announce|dispatch)-|cli-proxy$", name)
    ]
    assert len(writers) > 20
    for name in writers:
        condition = str(jobs[name].get("if", ""))
        if condition.startswith("${{ github.event_name == 'workflow_dispatch' && inputs.plugins-only &&"):
            continue
        assert "prepare" in ancestors(jobs, name), name
        opts_out = "always()" in condition or "!cancelled()" in condition
        assert not opts_out or PREPARE_OK in condition or VALIDATED in condition, name
    validate = jobs["validate-versions"]
    assert needs(validate) == ["prepare"]
    assert "always()" not in validate["if"]
    assert "!cancelled()" not in validate["if"]


def test_promotion_requires_swift_and_rechecks_the_live_release() -> None:
    job = publish_jobs()["promote-release"]
    assert f"({PREPARE_OUTPUTS}.release_swift != 'true' || needs.upload-swift-release.result == 'success')" in job["if"]
    (flip,) = [step for step in job["steps"] if step.get("id") == "flip"]
    for output in ("release_id", "tag_object", "swift_bundle_sha256", "swift_receipt_sha256", "release_swift"):
        assert f"{PREPARE_OUTPUTS}.{output}" in yaml.safe_dump(flip["env"]), output
    script = flip["run"]
    assert script.index("git/ref/tags/") < script.index("draft=false")
    assert script.index(".digest") < script.index("draft=false")
    assert "gh release edit" not in script


def test_prepare_rejects_tag_moving_options_on_real_runs() -> None:
    steps = publish_jobs()["prepare"]["steps"]
    reject = steps[0]
    assert reject["name"] == "Reject destructive release options"
    assert set(reject["env"]) == {
        "REPUBLISH",
        "FORCE_REPUBLISH",
        "PAYLOAD_REPUBLISH",
        "PAYLOAD_FORCE_REPUBLISH",
        "DRY_RUN",
        "PAYLOAD_DRY_RUN",
    }
    assert "exit 1" in reject["run"]
    assert not uses(publish_jobs()["prepare"], "xberg-io/actions/retag-for-republish")


def test_prepare_calls_the_shared_workflow_with_this_repository_bundle() -> None:
    document = workflow("prepare-swift-release.yaml")
    assert document["name"] == "Prepare Swift release"
    assert document[True] == {
        "workflow_dispatch": {
            "inputs": {
                "dry-run": {
                    "description": "Build, pin and verify, and report the ref plan, without pushing, tagging or releasing",
                    "type": "boolean",
                    "default": False,
                }
            }
        }
    }
    assert document["concurrency"] == {"group": "prepare-swift-release", "cancel-in-progress": False}
    assert document["permissions"] == {"contents": "read"}
    assert list(document["jobs"]) == ["swift"]
    assert document["jobs"]["swift"] == {
        "uses": "xberg-io/actions/.github/workflows/reusable-swift-release-prepare.yml@v1",
        "secrets": {
            "app-id": "${{ secrets.BOT_APP_ID }}",
            "app-private-key": "${{ secrets.BOT_APP_PRIVATE_KEY }}",
        },
        "with": {
            "dry-run": "${{ inputs.dry-run }}",
            "crate-name": "liter-llm-swift",
            "artifact-name": BUNDLE.removesuffix(".artifactbundle.zip"),
            "header-path": "target/release/build/liter-llm-swift-*/out",
        },
    }
