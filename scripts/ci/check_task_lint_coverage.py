#!/usr/bin/env python3
"""Assert that poly surveys every mise task file and skips none of them.

The task automation lives in ``mise.toml`` and under ``mise-tasks/`` (included
TOML task files and ``.sh`` file tasks). poly lints and formats all of them, but
a file poly declines to parse is reported as a skip, and a newly skipped file
looks identical to one that was checked and found clean. The go-task YAML this
replaced was skipped for its Go template syntax; nothing in the mise files has a
reason to be skipped.

This check reads poly's ``--format json`` report, which names every file it
surveyed and, for each skip, the reason. So the assertion is made per file:

    no file in the scan set may be skipped,
    and every file in the scan set must appear in the report

Neither a count nor a list of expected filenames appears here: the scan set is
derived from the tree, so adding a task file widens it automatically.

Retire this in favour of a poly flag (``--deny-skips`` / ``--max-skips=N``) if
one ships; it was requested upstream. This is a workaround for a missing
strict mode, not a thing worth keeping on its own merits.

Usage:
    python3 scripts/ci/check_task_lint_coverage.py    # exit 1 on any skip or unsurveyed file
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

# ~keep `poly fmt --check` exits 1 for "these files would reformat", which is a
# real report this check deliberately ignores — it reads skip reasons, not
# formatting verdicts. Anything above that (2 = unreadable path argument, and
# every crash/abort path) means poly did NOT survey the scan set, so any report
# scraped from such a run describes nothing. Measured against poly directly;
# do not widen this set without re-measuring.
POLY_ALL_FORMATTED = 0
POLY_WOULD_REFORMAT = 1
POLY_SURVEYED_EXIT_CODES = frozenset({POLY_ALL_FORMATTED, POLY_WOULD_REFORMAT})


def scan_set() -> list[Path]:
    """The files handed to poly: mise.toml plus every TOML and shell task file under mise-tasks/."""
    tasks = ROOT / "mise-tasks"
    paths = sorted([*tasks.rglob("*.toml"), *tasks.rglob("*.sh")])
    root_config = ROOT / "mise.toml"
    if root_config.exists():
        paths.append(root_config)
    if not paths:
        raise SystemExit(f"no task files found under {ROOT}; the scan set cannot be empty")
    return paths


def poly_report(paths: list[Path]) -> list[dict]:
    """Run poly and return its per-file JSON report.

    ``--no-cache`` because a cache hit changes what poly reports, which is the
    exact trap that made an earlier version of this check give a false pass.

    The exit code is asserted *before* the report is parsed. A run that aborted
    can still have emitted a report for the prefix of the scan set it reached,
    and reading that would describe a survey that never finished — a pass
    earned by examining nothing.
    """
    poly = shutil.which("poly")
    if poly is None:
        raise SystemExit("poly is not on PATH; run `mise install` first")

    argv = [poly, "fmt", "--check", "--no-cache", "--format", "json", *[str(p) for p in paths]]
    result = subprocess.run(argv, capture_output=True, text=True, check=False)

    if result.returncode not in POLY_SURVEYED_EXIT_CODES:
        raise SystemExit(
            f"poly exited {result.returncode}, so it did not survey the scan set and any\n"
            "report scraped from this run would be meaningless. Refusing to report a skip\n"
            f"verdict for a failed run.\ncommand: {' '.join(argv)}\n"
            f"poly stdout:\n{result.stdout}\npoly stderr:\n{result.stderr}"
        )

    # ~keep poly prints its human skip lines after the JSON document, so decode
    # the leading value rather than handing the whole stream to json.loads.
    try:
        report, _ = json.JSONDecoder().raw_decode(result.stdout.lstrip())
    except ValueError as exc:
        raise SystemExit(
            "could not parse poly's JSON report; its output format changed.\n"
            "This check must fail loudly rather than assume zero skips.\n"
            f"parse error: {exc}\npoly said:\n{result.stdout}\n{result.stderr}"
        ) from exc
    return report_rows(report)


def report_rows(report: object) -> list[dict]:
    """Normalize supported Poly reports and reject formatter failures."""
    if isinstance(report, dict):
        if report.get("errors") or report.get("summary", {}).get("errored"):
            raise SystemExit(f"poly reported formatter errors:\n{report}")
        report = report.get("results")
    if not isinstance(report, list) or any(not isinstance(entry, dict) for entry in report):
        raise SystemExit(f"poly's JSON report must contain a per-file results list:\n{report}")
    if any(entry.get("error") for entry in report):
        raise SystemExit(f"poly reported formatter errors:\n{report}")
    return report


def _display(path: str) -> str:
    """Path as written in the repo, whatever form poly echoed back."""
    resolved = Path(path).resolve()
    return str(resolved.relative_to(ROOT)) if resolved.is_relative_to(ROOT) else str(resolved)


def main() -> int:
    """Assert poly skipped no task file, and nothing went unsurveyed.

    The printed lines are this check's machine-readable result, not incidental
    logging, so `print` is the correct surface here (ruff T201 suppressed per
    call for that reason).
    """
    paths = scan_set()
    report = poly_report(paths)

    # ~keep poly echoes back whatever path form it was handed, so both sides are
    # resolved before comparison rather than assumed to be repo-relative.
    reported = {Path(entry["path"]).resolve() for entry in report if entry.get("path")}
    unsurveyed = sorted(str(p.relative_to(ROOT)) for p in paths if p.resolve() not in reported)

    skipped = sorted((_display(entry["path"]), entry["skipped"]) for entry in report if entry.get("skipped"))

    if not unsurveyed and not skipped:
        print(f"OK: {len(paths)} task file(s) linted, none skipped.")  # noqa: T201
        return 0

    print("Task lint coverage changed unexpectedly:", file=sys.stderr)  # noqa: T201
    if unsurveyed:
        print(  # noqa: T201
            f"  {len(unsurveyed)} file(s) in the scan set are absent from poly's report, so they\n"
            "  were never surveyed and this run proves nothing about them:",
            file=sys.stderr,
        )
        for path in unsurveyed:
            print(f"    {path}", file=sys.stderr)  # noqa: T201
    if skipped:
        print(  # noqa: T201
            f"  {len(skipped)} file(s) were skipped and are therefore silently unlinted —\n"
            "  find out why before assuming this is benign:",
            file=sys.stderr,
        )
        for path, reason in skipped:
            print(f"    {path}: {reason}", file=sys.stderr)  # noqa: T201
    return 1


if __name__ == "__main__":
    sys.exit(main())
