"""Keep package identities separate from an implicit dry-run release tag."""

import os
import re
import tomllib
from pathlib import Path


def release_versions(
    version: str,
    canonical: str,
    *,
    tag: str,
    dry_run: str,
    is_tag: str,
    requested_tag: str,
    target_sha: str,
) -> dict[str, str]:
    synthesized = f"{canonical}-dryrun-{target_sha[:7]}"
    implicit_dry_run = (
        dry_run == "true"
        and is_tag == "true"
        and not requested_tag
        and re.fullmatch(r"[0-9a-f]{40}", target_sha) is not None
        and tag == f"v{synthesized}"
        and version == synthesized
    )
    artifact = canonical if implicit_dry_run else version
    pypi = re.sub(r"-alpha\.?([0-9]+)$", r"a\1", artifact)
    pypi = re.sub(r"-beta\.?([0-9]+)$", r"b\1", pypi)
    pypi = re.sub(r"-([A-Za-z]+)\.?([0-9]+)$", r"\1\2", pypi)
    gem = re.sub(r"^([^-]+)-(.*)$", r"\1.pre.\2", artifact).replace("_", ".").replace("-", ".")
    return {"artifact_version": artifact, "pypi_version": pypi, "gem_version": gem}


def main() -> None:
    with Path("Cargo.toml").open("rb") as manifest:
        canonical = tomllib.load(manifest)["workspace"]["package"]["version"]
    versions = release_versions(
        os.environ["VERSION"],
        canonical,
        tag=os.environ["RELEASE_TAG"],
        dry_run=os.environ["DRY_RUN"],
        is_tag=os.environ["IS_TAG"],
        requested_tag=os.environ["REQUESTED_TAG"],
        target_sha=os.environ["TARGET_SHA"],
    )
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as output:
        output.write("".join(f"{key}={value}\n" for key, value in versions.items()))


if __name__ == "__main__":
    main()
