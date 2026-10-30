"""Regression coverage for the ai-rulez never-pinned check (org ADR-0015)."""

from __future__ import annotations

from pathlib import Path

import check_ai_rulez_version as check
import pytest


class TestWorkflowPins:
    """Every workflow input must be latest."""

    def test_workflow_inputs_are_collected(self, tmp_path: Path) -> None:
        (tmp_path / "a.yaml").write_text('with:\n  ai-rulez-version: "latest"\n')
        (tmp_path / "b.yml").write_text("with:\n  ai-rulez-version: 5.7.0 # old\n")
        (tmp_path / "c.yml").write_text("name: none\n")
        assert check.workflow_pins(tmp_path) == {"a.yaml": ["latest"], "b.yml": ["5.7.0"]}

    def test_latest_inputs_pass(self) -> None:
        check.check_workflows({"a.yaml": ["latest", "latest"]})

    def test_exact_pin_names_the_file(self) -> None:
        with pytest.raises(SystemExit, match=r"b\.yml: 5\.7\.0"):
            check.check_workflows({"a.yaml": ["latest"], "b.yml": ["5.7.0"]})

    def test_no_inputs_fails(self) -> None:
        with pytest.raises(SystemExit, match="No ai-rulez-version"):
            check.check_workflows({})

    def test_setup_mise_install_names_the_file(self) -> None:
        with pytest.raises(SystemExit, match=r"setup-ai-rulez, not setup-mise: c\.yaml"):
            check.check_workflows({"a.yaml": ["latest"]}, ["c.yaml"])

    def test_setup_mise_installs_are_collected(self, tmp_path: Path) -> None:
        (tmp_path / "a.yaml").write_text("with:\n  install-args: node github:Goldziher/ai-rulez\n")
        (tmp_path / "b.yml").write_text("with:\n  install-args: node\n")
        assert check.mise_installs(tmp_path) == ["a.yaml"]

    def test_unknown_mode_fails(self) -> None:
        with pytest.raises(SystemExit, match="usage"):
            check.main(["check_ai_rulez_version.py", "installed"])

    def test_repository_inputs_are_latest(self) -> None:
        pins = check.workflow_pins()
        assert sum(len(values) for values in pins.values()) == 8
        check.check_workflows(pins, check.mise_installs())


class TestMisePin:
    """No mise config pins ai-rulez."""

    def test_pin_names_the_file(self, tmp_path: Path) -> None:
        (tmp_path / "mise.toml").write_text('[tools]\nnode = "26.12.1"\n')
        (tmp_path / "mise.ci.toml").write_text('[tools]\n"github:Goldziher/ai-rulez" = "5.9.3"\n')
        with pytest.raises(SystemExit, match=r"never pinned; remove it from: mise\.ci\.toml"):
            check.check_mise(tmp_path)

    def test_no_mise_config_fails(self, tmp_path: Path) -> None:
        with pytest.raises(SystemExit, match=r"No mise\*\.toml"):
            check.check_mise(tmp_path)

    def test_repository_has_no_pin(self) -> None:
        check.check_mise()


def write_hooks(root: Path, alef: str, poly: str) -> None:
    source = 'id = "ai-rulez"\ngit = "https://github.com/Goldziher/ai-rulez.git"\nrevision = "{}"\n'
    (root / "alef.toml").write_text("[[workspace.poly.hooks-sources]]\n" + source.format(alef))
    (root / "poly.toml").write_text("[[hooks.sources]]\n" + source.format(poly))


class TestHookSource:
    """The ai-rulez hook source follows the floating v5 tag."""

    def test_floating_tag_passes(self, tmp_path: Path) -> None:
        write_hooks(tmp_path, "v5", "v5")
        check.check_hooks(tmp_path)

    def test_exact_tag_names_the_file(self, tmp_path: Path) -> None:
        write_hooks(tmp_path, "v5", "v5.9.2")
        with pytest.raises(SystemExit, match=r"must follow v5: poly\.toml: v5\.9\.2"):
            check.check_hooks(tmp_path)

    def test_missing_source_fails(self, tmp_path: Path) -> None:
        (tmp_path / "alef.toml").write_text("[workspace]\n")
        (tmp_path / "poly.toml").write_text("[hooks]\n")
        with pytest.raises(SystemExit, match="found 0"):
            check.check_hooks(tmp_path)

    def test_repository_follows_floating_tag(self) -> None:
        check.check_hooks()
