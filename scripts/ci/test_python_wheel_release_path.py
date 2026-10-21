"""The release path of the Python wheels: the workflows, jobs and task that build, check, upload and test them.

Each test reads the parsed workflows and the Taskfile and compares whole key sets and whole values of the jobs and
steps it guards, so a key no test expects on a guarded job or step is an error, not a value the test ignores.
Comments and descriptions are labels and no test reads them. A step name is read only where a test expects one
named step, as the package listing of the PyPI job. The wheel fixtures come from the checker's own tests.
"""

import posixpath
import re
import shlex
import shutil
import subprocess
import sys
from collections.abc import Iterator
from fnmatch import fnmatchcase
from pathlib import Path

import test_python_wheel_check as checker_tests

ROOT = Path(__file__).resolve().parents[2]
# This file and the checker's tests: the release regression task must run both.
WHEEL_TEST_FILES = tuple(
    Path(file).resolve().relative_to(ROOT).as_posix() for file in (__file__, checker_tests.__file__)
)


def wheel_job() -> dict:
    return workflow("python-wheels.yaml")["jobs"]["build"]


def workflow(name: str) -> dict:
    import yaml

    return yaml.safe_load((ROOT / ".github/workflows" / name).read_text())


def test_release_builds_an_intel_macos_wheel_on_the_arm64_runner() -> None:
    legs = {leg["label"]: leg for leg in wheel_job()["strategy"]["matrix"]["include"]}
    intel = legs["macos-x86_64"]
    assert intel["runner"] == "macos-latest"
    assert intel["cibw_archs_macos"] == "x86_64"
    assert intel["wheel_macho_arch"] == "x86_64"
    assert intel["wheel_platform"] == "macosx_11_0_x86_64"
    assert "rustup target add x86_64-apple-darwin" in intel["before_build_macos"]
    build = next(step for step in wheel_job()["steps"] if "build-python-wheels" in step.get("uses", ""))
    assert "macosx_x86_64" not in build["with"]["cibw-skip"]
    assert "MACOSX_DEPLOYMENT_TARGET=11.0" in build["with"]["cibw-environment-macos"]


def test_release_and_pull_request_check_build_wheels_with_the_same_workflow() -> None:
    release = workflow("publish.yaml")["jobs"]["build-python-wheels"]
    check = workflow("ci-python-wheels.yaml")["jobs"]["wheels"]
    assert release["uses"] == "./.github/workflows/python-wheels.yaml"
    assert check["uses"] == "./.github/workflows/python-wheels.yaml"


RELEASE_WHEEL_LEGS = {
    "linux-x86_64": ("ubuntu-latest", "auto", "manylinux.*_x86_64", None),
    "linux-aarch64": ("ubuntu-24.04-arm", "auto", "manylinux.*_aarch64", None),
    "macos-arm64": ("macos-latest", "arm64", "macosx_11_0_arm64", "arm64"),
    "macos-x86_64": ("macos-latest", "x86_64", "macosx_11_0_x86_64", "x86_64"),
    "windows-x86_64": ("windows-latest", "auto", "win_amd64", None),
}


def test_release_job_builds_all_five_wheels_for_the_tag_and_uploads_them() -> None:
    release = workflow("publish.yaml")["jobs"]["build-python-wheels"]
    assert release["uses"] == "./.github/workflows/python-wheels.yaml"
    assert release["needs"] == ["prepare", "registry-results"]
    assert release["with"] == {"checkout_ref": "${{ needs.prepare.outputs.checkout_ref }}"}
    assert release["if"] == (
        "${{ !inputs.plugins-only && (always() && needs.prepare.outputs.release_python == 'true'"
        " && (needs.registry-results.outputs.pypi != 'true' || needs.prepare.outputs.force_republish == 'true')) }}"
    )

    job = wheel_job()
    legs = {
        leg["label"]: (leg["runner"], leg["cibw_archs_macos"], leg["wheel_platform"], leg.get("wheel_macho_arch"))
        for leg in job["strategy"]["matrix"]["include"]
    }
    assert legs == RELEASE_WHEEL_LEGS
    assert job["env"]["CIBW_ARCHS_MACOS"] == "${{ matrix.cibw_archs_macos }}"

    checkout = next(step for step in job["steps"] if "actions/checkout" in step.get("uses", ""))
    assert checkout["with"] == {"ref": "${{ inputs.checkout_ref }}"}
    upload = next(step for step in job["steps"] if "upload-artifact" in step.get("uses", ""))
    assert upload["with"]["path"] == "./wheelhouse/*.whl"


def test_pull_request_wheel_check_watches_the_crates_the_wheel_is_built_from() -> None:
    paths = workflow("ci-python-wheels.yaml")[True]["pull_request"]["paths"]
    for watched in ("crates/liter-llm/**", "crates/liter-llm-py/**", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        assert watched in paths


def test_pull_request_wheel_check_runs_only_in_this_repository() -> None:
    assert "github.repository == 'xberg-io/liter-llm'" in workflow("ci-python-wheels.yaml")["jobs"]["wheels"]["if"]


CONCRETE_TAGS = {
    "linux-x86_64": "manylinux_2_28_x86_64",
    "linux-aarch64": "manylinux_2_28_aarch64",
    "macos-arm64": "macosx_11_0_arm64",
    "macos-x86_64": "macosx_11_0_x86_64",
    "windows-x86_64": "win_amd64",
}


def run_check_step(leg: dict, wheel_dir: Path) -> subprocess.CompletedProcess[str]:
    """Run the check step of the wheel job as the workflow wires it, with the leg's matrix values."""
    step = next(s for s in wheel_job()["steps"] if "check_python_wheel.py" in s.get("run", ""))
    env = {}
    for name, expression in step["env"].items():
        assert expression.startswith("${{ matrix."), expression
        assert expression.endswith(" }}"), expression
        env[name] = str(leg.get(expression[len("${{ matrix.") : -len(" }}")], ""))
    # The Windows runner's bash has `python` and no `python3`, so the PATH holds only `python`.
    bin_dir = wheel_dir / "bin"
    bin_dir.mkdir()
    (bin_dir / "python").symlink_to(sys.executable)
    (wheel_dir / "scripts").symlink_to(ROOT / "scripts")
    wheelhouse = wheel_dir / "wheelhouse"
    wheelhouse.mkdir()
    for wheel in wheel_dir.glob("*.whl"):
        wheel.rename(wheelhouse / wheel.name)
    bash = shutil.which("bash")
    assert bash
    return subprocess.run(
        [bash, "-c", step["run"]],
        cwd=wheel_dir,
        env={"PATH": str(bin_dir), **env},
        capture_output=True,
        text=True,
        check=False,
    )


def test_check_step_gives_each_leg_its_own_tag_and_architecture(tmp_path: Path) -> None:
    """Drives the step the way the release runs it: each leg rejects every other leg's tag, the Intel leg arm64 code."""
    legs = {leg["label"]: leg for leg in wheel_job()["strategy"]["matrix"]["include"]}
    assert set(legs) == set(CONCRETE_TAGS)
    for label, leg in legs.items():
        expected = {"x86_64": checker_tests.X86_64, "arm64": checker_tests.ARM64}.get(leg.get("wheel_macho_arch"))
        code = checker_tests.thin(expected) if expected else checker_tests.LINUX_ELF
        good = tmp_path / f"good-{label}"
        good.mkdir()
        checker_tests.make_wheel(good, CONCRETE_TAGS[label], {"liter_llm/_internal_bindings.abi3.so": code})
        result = run_check_step(leg, good)
        assert result.returncode == 0, (label, result.stdout + result.stderr)
        for other_label, other_tag in CONCRETE_TAGS.items():
            if other_label == label:
                continue
            foreign = tmp_path / f"tag-{other_label}-on-{label}"
            foreign.mkdir()
            checker_tests.make_wheel(foreign, other_tag, {"liter_llm/_internal_bindings.abi3.so": code})
            refusal = 2 if other_tag.startswith("macosx_") and not expected else 1
            assert run_check_step(leg, foreign).returncode == refusal, (label, other_label)
        if expected:
            wrong = tmp_path / f"wrong-{label}"
            wrong.mkdir()
            other = checker_tests.ARM64 if expected == checker_tests.X86_64 else checker_tests.X86_64
            checker_tests.make_wheel(
                wrong, CONCRETE_TAGS[label], {"liter_llm/_internal_bindings.abi3.so": checker_tests.thin(other)}
            )
            assert run_check_step(leg, wrong).returncode == 1, label


def test_wheel_build_inputs_are_pinned() -> None:
    job = wheel_job()
    build = next(s for s in job["steps"] if "build-python-wheels" in s.get("uses", ""))["with"]
    assert build["cibw-build"] == "cp310-*"
    assert "--compatibility linux" in build["cibw-environment"]
    assert (ROOT / build["package-dir"] / "pyproject.toml").is_file()
    assert job["timeout-minutes"] == 90
    upload = next(s for s in job["steps"] if "upload-artifact" in s.get("uses", ""))
    assert upload["with"]["retention-days"] == 7
    assert upload["with"]["if-no-files-found"] == "error"


# The release path rests on five invariants, one test each.

PYPI_PUBLISH_CONDITION = (
    "${{ !inputs.plugins-only && (always() && needs.prepare.outputs.release_python == 'true'"
    " && needs.validate-versions.result == 'success' && needs.build-python-wheels.result == 'success'"
    " && needs.build-python-sdist.result == 'success'"
    " && (needs.registry-results.outputs.pypi != 'true' || needs.prepare.outputs.force_republish == 'true')) }}"
)
LIST_PACKAGES_RUN = 'echo "Packages in dist:"\nls -lh dist/ 2>/dev/null || echo "No packages found"\n'


def strings(node: object) -> Iterator[str]:
    """Every key and every string value in a parsed workflow, at any depth."""
    if isinstance(node, dict):
        for key, value in node.items():
            yield str(key)
            yield from strings(value)
    elif isinstance(node, list):
        for item in node:
            yield from strings(item)
    elif isinstance(node, str):
        yield node


def step_index(steps: list[dict], uses: str) -> int:
    (index,) = (i for i, step in enumerate(steps) if uses in step.get("uses", ""))
    return index


def check_step_index(steps: list[dict]) -> int:
    (index,) = (i for i, step in enumerate(steps) if "check_python_wheel.py" in step.get("run", ""))
    return index


def publish_pypi_job() -> dict:
    return workflow("publish.yaml")["jobs"]["publish-pypi"]


def release_run_uploads() -> Iterator[tuple[str, str]]:
    """Every artifact upload in the release run as (job, artifact name), reusable workflows included."""
    for job_id, job in workflow("publish.yaml")["jobs"].items():
        jobs = {job_id: job}
        if job.get("uses", "").startswith("./.github/workflows/"):
            called = workflow(job["uses"].removeprefix("./.github/workflows/"))["jobs"]
            jobs = {f"{job_id}/{called_id}": called_job for called_id, called_job in called.items()}
        for name, called_job in jobs.items():
            for step in called_job.get("steps", []):
                if "actions/upload-artifact" in step.get("uses", ""):
                    yield name, step["with"].get("name", "artifact")


def test_invariant_check_step_always_runs_and_its_failure_fails_the_release() -> None:
    """No key on the check step, the wheel job, its workflow or a caller can skip the check or absorb its exit code."""
    reusable = workflow("python-wheels.yaml")
    assert set(reusable) == {"name", True, "permissions", "jobs"}
    assert set(reusable["jobs"]) == {"build"}
    job = reusable["jobs"]["build"]
    assert set(job) == {"name", "runs-on", "timeout-minutes", "env", "strategy", "steps"}
    assert job["strategy"] == {"fail-fast": False, "matrix": {"include": job["strategy"]["matrix"]["include"]}}
    check = job["steps"][check_step_index(job["steps"])]
    assert set(check) == {"name", "shell", "env", "run"}
    assert check["shell"] == "bash"
    assert set(check["env"]) == {"WHEEL_PLATFORM", "WHEEL_MACHO_ARCH"}

    assert set(workflow("publish.yaml")["jobs"]["build-python-wheels"]) == {"name", "needs", "if", "uses", "with"}
    assert set(workflow("ci-python-wheels.yaml")["jobs"]["wheels"]) == {"name", "if", "uses"}

    publish = publish_pypi_job()
    assert "build-python-wheels" in publish["needs"]
    assert publish["if"] == PYPI_PUBLISH_CONDITION


def test_invariant_pypi_publishes_exactly_the_files_the_check_step_read() -> None:
    """The upload follows the check and reads its directory; PyPI reads only the wheel and sdist artifacts."""
    steps = wheel_job()["steps"]
    check = check_step_index(steps)
    build = step_index(steps, "xberg-io/actions/build-python-wheels")
    assert build < check
    assert steps[build]["with"]["upload-artifact"] == "false"
    upload = step_index(steps, "actions/upload-artifact")
    assert upload == check + 1
    checked_dir = shlex.split(steps[check]["run"])[2]
    assert posixpath.normpath(steps[upload]["with"]["path"]) == posixpath.join(checked_dir, "*.whl")
    wheel_artifact = steps[upload]["with"]["name"]
    assert wheel_artifact == "python-wheel-${{ matrix.label }}"

    publish = publish_pypi_job()["steps"]
    download = step_index(publish, "actions/download-artifact")
    target = step_index(publish, "xberg-io/actions/publish-pypi")
    pull = publish[download]["with"]
    assert set(pull) == {"pattern", "path", "merge-multiple"}
    assert pull["merge-multiple"] is True
    assert posixpath.normpath(publish[target]["with"]["packages-dir"]) == posixpath.normpath(pull["path"])
    between = publish[download + 1 : target]
    assert [(step["name"], step.get("uses"), step["run"]) for step in between] == [
        ("List packages to publish", None, LIST_PACKAGES_RUN)
    ]

    pattern = pull["pattern"]
    prefix = pattern.removesuffix("*")
    assert prefix
    assert "*" not in prefix
    assert pattern == prefix + "*"
    for label in CONCRETE_TAGS:
        assert fnmatchcase(wheel_artifact.replace("${{ matrix.label }}", label), pattern), label
    reaching = set()
    for job, name in release_run_uploads():
        literal = name.split("${{")[0]
        if literal.startswith(prefix) or ("${{" in name and prefix.startswith(literal)):
            reaching.add((job, name))
    assert reaching == {("build-python-wheels/build", wheel_artifact), ("build-python-sdist", "python-sdist")}


def test_invariant_pypi_checks_every_file_against_the_release_version() -> None:
    """The publish guard compares each file with the PyPI spelling of the version the tag names."""
    publish = publish_pypi_job()
    assert "prepare" in publish["needs"]
    target = publish["steps"][step_index(publish["steps"], "xberg-io/actions/publish-pypi")]
    assert target["with"]["expected-version"] == "${{ needs.prepare.outputs.pypi_version }}"

    prepare = workflow("publish.yaml")["jobs"]["prepare"]
    assert prepare["outputs"]["pypi_version"] == "${{ steps.version_forms.outputs.pypi_version }}"
    assert prepare["outputs"]["checkout_ref"] == "${{ steps.meta.outputs.checkout_ref }}"
    steps = {step.get("id"): step for step in prepare["steps"]}
    assert set(steps["meta"]) == {"name", "id", "uses", "with"}
    forms = steps["version_forms"]
    assert set(forms) == {"name", "id", "shell", "env", "run"}
    assert forms["env"]["VERSION"] == "${{ steps.meta.outputs.version }}"
    assert forms["env"]["RELEASE_TAG"] == "${{ steps.meta.outputs.tag }}"
    assert forms["run"] == "task ci:release-version-forms"


def test_invariant_release_builds_one_wheel_per_tag_and_no_other() -> None:
    """Five legs, five tags, each leg's tag pattern accepts its own tag and no other leg's."""
    legs = wheel_job()["strategy"]["matrix"]["include"]
    labels = [leg["label"] for leg in legs]
    assert sorted(labels) == sorted(CONCRETE_TAGS)
    optional = {"wheel_macho_arch", "before_build_macos"}
    for leg in legs:
        assert (
            {"label", "runner", "cibw_archs_macos", "wheel_platform"}
            <= set(leg)
            <= {
                "label",
                "runner",
                "cibw_archs_macos",
                "wheel_platform",
                *optional,
            }
        )
        accepted = {label for label, tag in CONCRETE_TAGS.items() if re.fullmatch(leg["wheel_platform"], tag)}
        assert accepted == {leg["label"]}, leg["label"]
        assert (leg.get("wheel_macho_arch") is not None) == CONCRETE_TAGS[leg["label"]].startswith("macosx_")


def test_invariant_wheel_path_holds_no_permission_beyond_the_upload() -> None:
    """The build has read access only; the PyPI job has the OIDC token and no secret."""
    assert workflow("publish.yaml")["permissions"] == {"contents": "read"}
    reusable = workflow("python-wheels.yaml")
    pull_request = workflow("ci-python-wheels.yaml")
    assert "pull_request" in pull_request[True]
    for checked in (reusable, pull_request):
        assert checked["permissions"] == {"contents": "read"}
        for text in strings(checked):
            for word in ("secrets", "github.token", "GITHUB_TOKEN", "id-token", "twine", "gh-action-pypi-publish"):
                assert word not in text, (word, text)
            assert "maturin publish" not in text, text
            assert "maturin upload" not in text, text

    publish = publish_pypi_job()
    assert set(publish) == {"name", "needs", "if", "runs-on", "permissions", "timeout-minutes", "environment", "steps"}
    assert publish["permissions"] == {"contents": "read", "id-token": "write"}
    assert publish["environment"]["name"] == "pypi"
    for text in strings(publish):
        assert "secrets" not in text, text
        assert "github.token" not in text, text


# The tests below drive the workflow expressions the release evaluates, so they fail on any edit that changes
# what a leg, a dry run or a pull request does, not only on the edits that remove a key.

EXPRESSION_TOKEN = re.compile(r"\s*('[^']*'|==|!=|&&|\|\||[!()]|[A-Za-z_][\w.-]*)")


def evaluate(expression: str, context: dict[str, str]) -> object:
    """Evaluate a GitHub Actions expression; a name missing from the context fails the test."""
    body = expression.strip()
    if body.startswith("${{"):
        assert body.endswith("}}"), body
        body = body[3:-2]
    tokens, position = [], 0
    while body[position:].strip():
        match = EXPRESSION_TOKEN.match(body, position)
        assert match, body[position:]
        tokens.append(match.group(1))
        position = match.end()
    value, rest = disjunction(tokens, context)
    assert not rest, rest
    return value


def disjunction(tokens: list[str], context: dict[str, str]) -> tuple[object, list[str]]:
    left, tokens = conjunction(tokens, context)
    while tokens[:1] == ["||"]:
        right, tokens = conjunction(tokens[1:], context)
        left = left or right
    return left, tokens


def conjunction(tokens: list[str], context: dict[str, str]) -> tuple[object, list[str]]:
    left, tokens = comparison(tokens, context)
    while tokens[:1] == ["&&"]:
        right, tokens = comparison(tokens[1:], context)
        left = left and right
    return left, tokens


def comparison(tokens: list[str], context: dict[str, str]) -> tuple[object, list[str]]:
    left, tokens = operand(tokens, context)
    while tokens[:1] in (["=="], ["!="]):
        right, rest = operand(tokens[1:], context)
        left, tokens = (left == right) if tokens[0] == "==" else (left != right), rest
    return left, tokens


def operand(tokens: list[str], context: dict[str, str]) -> tuple[object, list[str]]:
    token, rest = tokens[0], tokens[1:]
    if token == "!":
        value, rest = operand(rest, context)
        return not value, rest
    if token == "(":
        value, rest = disjunction(rest, context)
        assert rest[:1] == [")"], rest
        return value, rest[1:]
    if token.startswith("'"):
        return token[1:-1], rest
    if token in {"true", "false"}:
        return token == "true", rest
    return context[token], rest


def resolve(value: str, context: dict[str, str]) -> object:
    """A workflow value: text outside `${{ }}` is a literal, the whole of one is an expression."""
    return evaluate(value, context) if value.strip().startswith("${{") else value


def matrix_context(leg: dict) -> dict[str, str]:
    """A leg's matrix values; a key that another leg sets and this one does not is empty, as in a real run."""
    names = {name for other in wheel_job()["strategy"]["matrix"]["include"] for name in other}
    return {f"matrix.{name}": str(leg.get(name, "")) for name in names}


RELEASE_REF = "refs/tags/v9.9.9"
EXPRESSION = re.compile(r"\$\{\{(.*?)\}\}", re.DOTALL)


def render(node: object, context: dict[str, str]) -> object:
    """A parsed workflow value as a run sees it: every `${{ }}` evaluated, a whole expression keeping its type."""
    if isinstance(node, dict):
        return {key: render(value, context) for key, value in node.items()}
    if isinstance(node, list):
        return [render(item, context) for item in node]
    if not isinstance(node, str):
        return node
    if node.count("${{") == 1 and EXPRESSION.fullmatch(node.strip()):
        return evaluate(node, context)
    return EXPRESSION.sub(lambda match: str(evaluate(match.group(1), context)), node)


def released(reference: str) -> str:
    """An action reference with its release tag folded, so a major-version bump keeps the pins and `@main` does not."""
    return re.sub(r"@v\d+(\.\d+)*$", "@vN", reference)


def without_names(step: dict) -> dict:
    return {key: value for key, value in step.items() if key != "name"}


def pinned_steps(steps: list[dict]) -> list[dict]:
    """The steps of a job as a pin holds them: names dropped, action release tags folded."""
    folded = [without_names(step) for step in steps]
    for step in folded:
        if "uses" in step:
            step["uses"] = released(step["uses"])
    return folded


DEFAULT_MACOS_BEFORE_BUILD = "pip install maturin uv && source ~/.cargo/env"
MACOS_BEFORE_BUILD = {
    "macos-x86_64": DEFAULT_MACOS_BEFORE_BUILD + " && rustup target add x86_64-apple-darwin",
}
WHEEL_BUILD_INPUTS = {
    "package-dir": "packages/python",
    "cibw-build": "cp310-*",
    "cibw-skip": "*-musllinux* *-win32 *-manylinux_i686",
    "cibw-environment": 'MATURIN_BUILD_ARGS="--compatibility linux" CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=cc',
    "cibw-environment-macos": 'MATURIN_BUILD_ARGS="" CARGO_BUILD_LOCKED=true MACOSX_DEPLOYMENT_TARGET=11.0',
    "upload-artifact": "false",
}
CHECK_RUN = 'python scripts/ci/check_python_wheel.py wheelhouse "$WHEEL_PLATFORM" "$WHEEL_MACHO_ARCH"'


def expected_leg(label: str) -> dict:
    """The wheel job of one leg as the release runs it: every key and every value, names aside."""
    runner, archs, platform, macho_arch = RELEASE_WHEEL_LEGS[label]
    return {
        "runs-on": runner,
        "timeout-minutes": 90,
        "env": {"CIBW_ARCHS_MACOS": archs},
        "steps": [
            {"uses": "actions/checkout@vN", "with": {"ref": RELEASE_REF}},
            {
                "uses": "xberg-io/actions/build-python-wheels@vN",
                "with": {
                    **WHEEL_BUILD_INPUTS,
                    "cibw-before-build-macos": MACOS_BEFORE_BUILD.get(label, DEFAULT_MACOS_BEFORE_BUILD),
                },
            },
            {
                "shell": "bash",
                "env": {"WHEEL_PLATFORM": platform, "WHEEL_MACHO_ARCH": macho_arch or ""},
                "run": CHECK_RUN,
            },
            {
                "uses": "actions/upload-artifact@vN",
                "with": {
                    "name": f"python-wheel-{label}",
                    "path": "./wheelhouse/*.whl",
                    "retention-days": 7,
                    "if-no-files-found": "error",
                },
            },
        ],
    }


def test_every_leg_runs_every_release_step_as_pinned() -> None:
    """Each leg's job and steps, rendered with its matrix values, equal the pinned leg: a skipped, absorbed or changed
    step fails, and a new leg falls under the same pins once it has a row in the leg table."""
    job = wheel_job()
    legs = job["strategy"]["matrix"]["include"]
    assert sorted(leg["label"] for leg in legs) == sorted(RELEASE_WHEEL_LEGS)
    for leg in legs:
        context = {**matrix_context(leg), "inputs.checkout_ref": RELEASE_REF}
        rendered = render({key: value for key, value in job.items() if key not in {"name", "strategy"}}, context)
        assert isinstance(rendered, dict)
        rendered["steps"] = pinned_steps(rendered["steps"])
        assert rendered == expected_leg(leg["label"]), leg["label"]


RECHECK_CONDITION = "steps.recheck.outputs.exists != 'true' || needs.prepare.outputs.force_republish == 'true'"


def test_pypi_job_steps_cannot_be_skipped_or_absorb_a_failure() -> None:
    """Every step of the PyPI job, names aside: no step may be skipped by a new condition or absorb its failure."""
    publish = publish_pypi_job()
    assert publish["needs"] == [
        "prepare",
        "validate-versions",
        "registry-results",
        "build-python-wheels",
        "build-python-sdist",
    ]
    assert publish["runs-on"] == "ubuntu-latest"
    assert publish["environment"] == {"name": "pypi", "url": "https://pypi.org/p/liter-llm"}
    steps = pinned_steps(publish["steps"])
    directory = steps[1].get("with", {}).get("path")
    assert isinstance(directory, str)
    assert directory
    published = steps[3].get("with", {}).get("packages-dir")
    assert isinstance(published, str)
    assert posixpath.normpath(published) == posixpath.normpath(directory)
    assert steps == [
        {
            "uses": "xberg-io/actions/check-registry@vN",
            "id": "recheck",
            "with": {"registry": "pypi", "package": "liter-llm", "version": "${{ needs.prepare.outputs.version }}"},
        },
        {
            "if": RECHECK_CONDITION,
            "uses": "actions/download-artifact@vN",
            "with": {"pattern": "python-*", "path": directory, "merge-multiple": True},
        },
        {"if": RECHECK_CONDITION, "run": LIST_PACKAGES_RUN, "shell": "bash"},
        {
            "if": f"needs.prepare.outputs.dry_run != 'true' && ({RECHECK_CONDITION})",
            "uses": "xberg-io/actions/publish-pypi@vN",
            "with": {
                "packages-dir": published,
                "dry-run": "${{ needs.prepare.outputs.dry_run }}",
                "expected-version": "${{ needs.prepare.outputs.pypi_version }}",
            },
        },
    ]


def test_prepare_hands_the_wheel_path_its_own_release_facts() -> None:
    """The outputs the wheel build and the PyPI job read come from the metadata step, not from a fixed value, and
    the metadata step is told that Python is a release target."""
    prepare = workflow("publish.yaml")["jobs"]["prepare"]
    for name in ("tag", "checkout_ref", "version", "dry_run", "force_republish", "release_python"):
        assert prepare["outputs"][name] == f"${{{{ steps.meta.outputs.{name} }}}}", name
    (meta,) = (step for step in pinned_steps(prepare["steps"]) if step.get("id") == "meta")
    assert set(meta) == {"id", "uses", "with"}
    assert meta["uses"] == "xberg-io/actions/prepare-release-metadata@vN"
    assert set(meta["with"]) == {"available-targets"}
    assert "python" in meta["with"]["available-targets"].split(",")


CHECKED_PATHS = [
    "packages/python/**",
    "crates/liter-llm/**",
    "crates/liter-llm-py/**",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "scripts/ci/check_python_wheel.py",
    ".github/workflows/python-wheels.yaml",
    ".github/workflows/ci-python-wheels.yaml",
]


def test_pull_request_check_runs_on_every_pull_request_to_main_that_edits_the_wheel_path() -> None:
    """The check's triggers and its one job, whole: another base branch or a narrower filter fails."""
    check = workflow("ci-python-wheels.yaml")
    assert check[True] == {"pull_request": {"branches": ["main"], "paths": CHECKED_PATHS}, "workflow_dispatch": None}
    assert without_names(check["jobs"]["wheels"]) == {
        "if": "github.repository == 'xberg-io/liter-llm' && github.actor != 'dependabot[bot]'",
        "uses": "./.github/workflows/python-wheels.yaml",
    }


REGRESSIONS_PYTHON = "3.14"
RELEASE_REGRESSIONS_PYTEST = shlex.split(
    f"uv run --locked --only-group dev --python {REGRESSIONS_PYTHON} python -m pytest -q"
)
REGRESSIONS_INSTALL = (
    f"uv sync --locked --only-group dev --python {REGRESSIONS_PYTHON} --no-install-project --no-install-workspace"
)


def regression_task() -> dict:
    import yaml

    return yaml.safe_load((ROOT / "Taskfile.yml").read_text())["tasks"]["test:release-regressions"]


def test_release_regression_task_and_job_fail_when_this_file_fails() -> None:
    """The task runs pytest on named test files and nothing that absorbs its exit code. The CI job, pinned whole
    with names aside, checks out the pull request's own tree (its checkout step takes no ref), installs the Python
    the task runs, and cannot be skipped or absorb its failure."""
    task = regression_task()
    assert set(task) == {"desc", "cmds"}
    assert all(isinstance(command, str) for command in task["cmds"])
    (pytest_run,) = (shlex.split(command) for command in task["cmds"] if "pytest" in command)
    assert pytest_run[: len(RELEASE_REGRESSIONS_PYTEST)] == RELEASE_REGRESSIONS_PYTEST
    files = pytest_run[len(RELEASE_REGRESSIONS_PYTEST) :]
    for name in files:
        assert re.fullmatch(r"scripts/ci/test_\w+\.py", name), name
        assert (ROOT / name).is_file(), name
    assert set(WHEEL_TEST_FILES) <= set(files)

    job = workflow("ci-lint.yaml")["jobs"]["release-regressions"]
    assert set(job) == {"name", "runs-on", "steps"}
    assert job["runs-on"] == "ubuntu-latest"
    assert pinned_steps(job["steps"]) == [
        {"uses": "actions/checkout@vN"},
        {
            "uses": "xberg-io/actions/setup-python-env@vN",
            "with": {
                "python-version": REGRESSIONS_PYTHON,
                "cache-prefix": "release-regressions",
                "install-command": REGRESSIONS_INSTALL,
            },
        },
        {"uses": "xberg-io/actions/install-task@vN"},
        {"run": "task test:release-regressions", "shell": "bash"},
    ]


LINT_TRIGGER = {"branches": ["main"], "paths-ignore": ["README.md", "CHANGELOG.md", "LICENSE*"]}


def test_regression_job_starts_for_every_pull_request_that_edits_what_it_tests() -> None:
    """The lint workflow's path filters leave out documents only: no file this test file or the task reads is
    filtered, so a pull request that edits the wheel path or these tests runs the regression job."""
    assert workflow("ci-lint.yaml")[True] == {
        "push": LINT_TRIGGER,
        "pull_request": LINT_TRIGGER,
        "workflow_dispatch": None,
    }
    ignored = [github_path_pattern(path) for path in LINT_TRIGGER["paths-ignore"]]
    tested = {
        "Taskfile.yml",
        "scripts/ci/check_python_wheel.py",
        "packages/python/pyproject.toml",
        ".github/workflows/publish.yaml",
        ".github/workflows/python-wheels.yaml",
        ".github/workflows/ci-python-wheels.yaml",
        ".github/workflows/ci-lint.yaml",
        *(
            word
            for command in regression_task()["cmds"]
            for word in shlex.split(command)
            if word.startswith("scripts/")
        ),
    }
    assert len(tested) > 7
    for path in sorted(tested):
        assert (ROOT / path).is_file(), path
        assert not any(pattern.fullmatch(path) for pattern in ignored), path
    for document in ("README.md", "CHANGELOG.md", "LICENSE"):
        assert (ROOT / document).is_file(), document
        assert any(pattern.fullmatch(document) for pattern in ignored), document


WHEEL_WORKFLOW_INPUTS = {"checkout_ref": {"required": False, "default": "", "type": "string"}}


def without_descriptions(inputs: dict) -> dict:
    return {name: {key: value for key, value in spec.items() if key != "description"} for name, spec in inputs.items()}


def test_pull_request_check_builds_the_pull_request_and_the_release_builds_its_tag() -> None:
    """The check passes no input, so the default of `checkout_ref` decides what it builds: empty checks out the ref
    of the calling run, the merge commit of the pull request. The release passes the ref its metadata step resolved,
    and the checkout step of the wheel job takes exactly that ref."""
    reusable = workflow("python-wheels.yaml")
    inputs = reusable[True]["workflow_call"]["inputs"]
    assert reusable[True] == {"workflow_call": {"inputs": inputs}}
    assert without_descriptions(inputs) == WHEEL_WORKFLOW_INPUTS
    steps = wheel_job()["steps"]
    checkout = steps[step_index(steps, "actions/checkout")]
    assert set(checkout) == {"uses", "with"}

    assert "with" not in workflow("ci-python-wheels.yaml")["jobs"]["wheels"]
    defaults = {f"inputs.{name}": spec["default"] for name, spec in inputs.items()}
    assert render(checkout["with"], defaults) == {"ref": ""}

    release = workflow("publish.yaml")["jobs"]["build-python-wheels"]["with"]
    assert set(release) == set(inputs)
    passed = render(release, {"needs.prepare.outputs.checkout_ref": RELEASE_REF})
    assert isinstance(passed, dict)
    assert render(checkout["with"], {f"inputs.{name}": value for name, value in passed.items()}) == {"ref": RELEASE_REF}


def cancel_group(prefix: str) -> str:
    return f"{prefix}-${{{{ github.event.pull_request.number || github.ref }}}}"


def test_pull_request_run_cancels_only_an_older_run_of_the_same_pull_request() -> None:
    """The wheel check and the lint workflow (which runs the regression job) key their group on the pull request, so
    two pull requests never cancel each other and a push to main is its own group; the release runs one at a time
    and is never cancelled."""
    for name, prefix in (("ci-python-wheels.yaml", "ci-python-wheels"), ("ci-lint.yaml", "ci-lint")):
        checked = workflow(name)
        assert set(checked) == {"name", True, "concurrency", "permissions", "jobs"}, name
        assert checked["concurrency"] == {"group": cancel_group(prefix), "cancel-in-progress": True}, name
        groups = [
            render(checked["concurrency"]["group"], context)
            for context in (
                {"github.event.pull_request.number": "7", "github.ref": "refs/pull/7/merge"},
                {"github.event.pull_request.number": "8", "github.ref": "refs/pull/8/merge"},
                {"github.event.pull_request.number": "", "github.ref": "refs/heads/main"},
            )
        ]
        assert groups == [f"{prefix}-7", f"{prefix}-8", f"{prefix}-refs/heads/main"], name
    release = workflow("publish.yaml")
    assert set(release) == {"name", True, "concurrency", "permissions", "env", "jobs"}
    assert release["concurrency"] == {"group": "${{ github.workflow }}", "cancel-in-progress": False}


SDIST_BUILD_CONDITION = (
    "${{ !inputs.plugins-only && (!cancelled() && needs.crates-gate.outputs.ok == 'true'"
    " && needs.prepare.outputs.release_python == 'true'"
    " && (needs.registry-results.outputs.pypi != 'true' || needs.prepare.outputs.force_republish == 'true')) }}"
)


def test_sdist_job_builds_the_release_ref_and_uploads_under_the_name_the_pypi_job_reads() -> None:
    """PyPI publishes the sdist beside the wheels from one download, so the sdist job is pinned whole, names aside."""
    job = workflow("publish.yaml")["jobs"]["build-python-sdist"]
    assert set(job) == {"name", "needs", "if", "runs-on", "timeout-minutes", "steps"}
    assert job["needs"] == ["prepare", "registry-results", "crates-gate"]
    assert job["if"] == SDIST_BUILD_CONDITION
    assert job["runs-on"] == "ubuntu-latest"
    assert job["timeout-minutes"] == 30
    assert pinned_steps(job["steps"]) == [
        {"uses": "actions/checkout@vN", "with": {"ref": "${{ needs.prepare.outputs.checkout_ref }}"}},
        {"uses": "xberg-io/actions/setup-rust@vN", "with": {"use-sccache": False}},
        {"uses": "actions/setup-python@vN", "with": {"python-version": "3.14"}},
        {
            "uses": "xberg-io/actions/build-python-sdist@vN",
            "with": {
                "package-dir": "packages/python",
                "output-dir": "dist",
                "dry-run": "${{ needs.prepare.outputs.dry_run }}",
            },
        },
        {
            "uses": "actions/upload-artifact@vN",
            "with": {"name": "python-sdist", "path": "dist/*.tar.gz", "retention-days": 14},
        },
    ]


def test_each_leg_runs_on_the_runner_its_wheel_is_built_for() -> None:
    """A wheel job pinned to one runner would build every leg's wheel on that runner's operating system."""
    job = wheel_job()
    for leg in job["strategy"]["matrix"]["include"]:
        runner = resolve(job["runs-on"], matrix_context(leg))
        assert runner == RELEASE_WHEEL_LEGS[leg["label"]][0], leg["label"]


def test_every_macos_leg_builds_with_maturin_and_cargo_on_its_path() -> None:
    """The arm64 leg takes the default before-build command, the Intel leg its own; both need cargo's environment."""
    build = next(s for s in wheel_job()["steps"] if "build-python-wheels" in s.get("uses", ""))["with"]
    macos = [leg for leg in wheel_job()["strategy"]["matrix"]["include"] if leg["label"].startswith("macos-")]
    assert len(macos) == 2
    for leg in macos:
        command = resolve(build["cibw-before-build-macos"], matrix_context(leg))
        parts = [part.strip() for part in str(command).split("&&")]
        assert parts[:2] == ["pip install maturin uv", "source ~/.cargo/env"], (leg["label"], command)


def publish_to_pypi_step() -> dict:
    steps = publish_pypi_job()["steps"]
    return steps[step_index(steps, "xberg-io/actions/publish-pypi")]


def test_dry_run_never_uploads_to_pypi() -> None:
    """Two guards, each sufficient alone: the dry run skips the publish step, and the action is told it is a dry run."""
    step = publish_to_pypi_step()
    for exists in ("true", "false"):
        for force in ("true", "false"):
            context = {
                "needs.prepare.outputs.dry_run": "true",
                "steps.recheck.outputs.exists": exists,
                "needs.prepare.outputs.force_republish": force,
            }
            assert not evaluate(step["if"], context), (exists, force)
            assert resolve(step["with"]["dry-run"], context) == "true", (exists, force)
    release = {
        "needs.prepare.outputs.dry_run": "false",
        "steps.recheck.outputs.exists": "false",
        "needs.prepare.outputs.force_republish": "false",
    }
    assert evaluate(step["if"], release)
    assert resolve(step["with"]["dry-run"], release) == "false"


def test_pypi_job_has_time_to_upload_every_file() -> None:
    """PyPI takes files one at a time, so a job cancelled part way leaves a release with some wheels missing."""
    assert publish_pypi_job()["timeout-minutes"] == 15


def github_path_pattern(pattern: str) -> re.Pattern[str]:
    """A workflow path filter as a regex: `**` crosses directories, `*` and `?` do not."""
    assert not pattern.startswith("!"), pattern
    parts = re.split(r"(\*\*|\*|\?)", pattern)
    wildcards = {"**": ".*", "*": "[^/]*", "?": "[^/]"}
    return re.compile("".join(wildcards.get(part, re.escape(part)) for part in parts))


def test_pull_request_that_edits_the_wheel_path_runs_the_check() -> None:
    """An edit to the reusable workflow, the check script or the package must start the pull-request wheel check."""
    check_workflow = workflow("ci-python-wheels.yaml")
    patterns = [github_path_pattern(path) for path in check_workflow[True]["pull_request"]["paths"]]
    job = wheel_job()
    check = job["steps"][check_step_index(job["steps"])]
    build = next(s for s in job["steps"] if "build-python-wheels" in s.get("uses", ""))["with"]
    read_by_the_check = {
        ".github/workflows/ci-python-wheels.yaml",
        check_workflow["jobs"]["wheels"]["uses"].removeprefix("./"),
        shlex.split(check["run"])[1],
        posixpath.join(build["package-dir"], "pyproject.toml"),
    }
    assert len(read_by_the_check) == 4
    for path in sorted(read_by_the_check):
        assert (ROOT / path).is_file(), path
        assert any(pattern.fullmatch(path) for pattern in patterns), path
    assert not any(pattern.fullmatch("docs/index.md") for pattern in patterns)


def wheel_path_actions() -> Iterator[str]:
    """Every action or workflow reference the release and the pull-request check run to build and publish wheels."""
    yield workflow("publish.yaml")["jobs"]["build-python-wheels"]["uses"]
    yield workflow("ci-python-wheels.yaml")["jobs"]["wheels"]["uses"]
    for step in wheel_job()["steps"] + publish_pypi_job()["steps"]:
        if "uses" in step:
            yield step["uses"]


def test_wheel_path_runs_released_action_versions() -> None:
    """A branch ref such as `@main` lets a push to the action's repository change what the next release builds."""
    references = list(wheel_path_actions())
    assert len(references) >= 8
    for reference in references:
        assert reference.startswith("./") or re.fullmatch(r"[\w.-]+/[\w./-]+@v\d+(\.\d+)*", reference), reference


def test_wheel_path_starts_only_from_the_release_and_pull_requests() -> None:
    """A `pull_request_target` or `push` trigger would run the build with the base repository's secrets in reach."""
    assert set(workflow("python-wheels.yaml")[True]) == {"workflow_call"}
    assert set(workflow("ci-python-wheels.yaml")[True]) == {"pull_request", "workflow_dispatch"}


def test_release_regression_job_runs_this_file() -> None:
    """CI runs this file through the release regression task only; a deleted line would switch every test off.

    The test cannot see its own removal from CI: it fails when this file runs directly, as `pytest scripts/ci` does.
    """
    job = workflow("ci-lint.yaml")["jobs"]["release-regressions"]
    assert [step["run"] for step in job["steps"] if "run" in step] == ["task test:release-regressions"]
    pytest_runs = [shlex.split(command) for command in regression_task()["cmds"] if "pytest" in command]
    assert len(pytest_runs) == 1
    for name in WHEEL_TEST_FILES:
        assert name in pytest_runs[0], name
