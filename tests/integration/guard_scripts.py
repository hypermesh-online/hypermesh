#!/usr/bin/env python3
"""Bounded script integration fixtures; run with TMPDIR on caller-owned SSD storage.

TMPDIR=/usr/lib/hypermesh/verification/... python3 tests/integration/guard_scripts.py
TEST_GUARD_SOURCE optionally selects another snapshot for baseline regression runs.
No real large files, local hooks, Git operations or live disk-pressure changes.
"""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SOURCE = Path(os.environ.get("TEST_GUARD_SOURCE", Path(__file__).resolve().parents[2]))
SCRATCH = Path(os.environ["TMPDIR"]).resolve()
if not SCRATCH.is_absolute() or SCRATCH == Path("/tmp") or Path("/tmp") in SCRATCH.parents:
    raise RuntimeError("TMPDIR must select caller-owned non-/tmp fixture storage")


class GuardFixture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="guard-", dir=SCRATCH)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "scripts").mkdir()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}")

    def executable(self, name, text):
        path = self.bin / name
        path.write_text("#!/bin/sh\n" + text)
        path.chmod(0o755)

    def run_guard(self, name, *args):
        path = self.root / "scripts" / name
        shutil.copy2(SOURCE / "scripts" / name, path)
        return subprocess.run(
            ["bash", str(path), *args], env=self.env, cwd=self.root,
            text=True, capture_output=True, timeout=15,
        )

    def assert_result(self, result, code, marker=None):
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        self.assertEqual(result.stderr, "", result.stdout + result.stderr)
        if marker is not None:
            self.assertIn(marker, result.stdout)
        if code:
            self.assertNotIn("PASS:", result.stdout)


class BuildFixture(GuardFixture):
    def setUp(self):
        super().setUp()
        self.env.update(GUARD_SIZE_KB="1048576", GUARD_FREE_GB="200")
        self.executable("du", 'printf "%s\\t%s\\n" "$GUARD_SIZE_KB" "$2"\n')
        self.executable("df", 'printf "Avail\\n%sG\\n" "$GUARD_FREE_GB"\n')

    def target(self, path="target"):
        (self.root / path).mkdir(parents=True)


class BuildSizeTests(BuildFixture):
    def test_empty_inventory_is_clean(self):
        self.assert_result(self.run_guard("check-build-size.sh"), 0, "no target trees")

    def test_healthy_target_is_clean(self):
        self.target()
        self.assert_result(self.run_guard("check-build-size.sh"), 0, "PASS:")


class BuildFailureTests(BuildFixture):
    def test_single_oversized_target_is_rejected(self):
        self.target()
        self.env["GUARD_SIZE_KB"] = str(81 * 1048576)
        self.assert_result(self.run_guard("check-build-size.sh"), 1, "per-tree budget")

    def test_combined_budget_is_rejected(self):
        self.target()
        self.target(".claude/worktrees/peer/target")
        self.env["GUARD_SIZE_KB"] = str(75 * 1048576)
        self.assert_result(self.run_guard("check-build-size.sh"), 1, "over the 140 GiB")

    def test_low_free_space_is_rejected(self):
        self.target()
        self.env["GUARD_FREE_GB"] = "59"
        self.assert_result(self.run_guard("check-build-size.sh"), 1, "only 59 GiB free")


class BuildBoundaryTests(BuildFixture):
    def test_exact_boundaries_are_accepted(self):
        self.target()
        self.env.update(GUARD_SIZE_KB=str(80 * 1048576), GUARD_FREE_GB="60")
        self.assert_result(self.run_guard("check-build-size.sh"), 0, "PASS:")

    def test_quiet_healthy_target_is_silent(self):
        self.target()
        result = self.run_guard("check-build-size.sh", "--quiet")
        self.assert_result(result, 0)
        self.assertEqual(result.stdout, "")

    def test_quiet_oversized_target_still_fails(self):
        self.target()
        self.env["GUARD_SIZE_KB"] = str(81 * 1048576)
        result = self.run_guard("check-build-size.sh", "--quiet")
        self.assert_result(result, 1)
        self.assertEqual(result.stdout, "")


class StoragePolicyTests(unittest.TestCase):
    def test_scanner_has_no_hardcoded_tmp_probe(self):
        source = (SOURCE / "scripts/check-no-pos-magnitude.sh").read_text()
        self.assertNotIn('local probe="/tmp/', source)


class ProofFixture(GuardFixture):
    def setUp(self):
        super().setUp()
        source = (SOURCE / "scripts/check-no-pos-magnitude.sh").read_text()
        if 'local probe="/tmp/' in source:
            self.skipTest("unsafe baseline probe retained as static RED; never execute /tmp write")
        (self.root / "orphan.rs").write_text("fn healthy() {}\n")


class ProofScannerTests(ProofFixture):
    def test_unset_tmpdir_needs_no_fallback(self):
        self.env.pop("TMPDIR", None)
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "PASS:")
        self.assertFalse(list(self.root.rglob("*selftest*")))

    def test_explicit_tmpdir_has_no_probe_or_cleanup_residue(self):
        temp = self.root / "scratch with spaces"
        temp.mkdir()
        self.env["TMPDIR"] = str(temp)
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "PASS:")
        self.assertEqual(list(temp.iterdir()), [])

    def test_missing_tmpdir_is_not_created(self):
        temp = self.root / "missing"
        self.env["TMPDIR"] = str(temp)
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "PASS:")
        self.assertFalse(temp.exists())

    def test_forbidden_magnitude_is_detected_in_orphan(self):
        (self.root / "orphan.rs").write_text("fn f() { let stake_amount = 1; }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "stake_amount")
        self.assertFalse(list(self.root.rglob("*selftest*")))

    def test_capacity_gate_is_detected(self):
        (self.root / "orphan.rs").write_text("fn f() { if proof.space_proof.total_size == 0 {} }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "capacity never gates")

    def test_comments_and_string_literals_do_not_trigger(self):
        (self.root / "orphan.rs").write_text('// stake_amount\nfn f() { let x = "difficulty"; }\n')
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "PASS:")


class ProofFailureTests(ProofFixture):
    def test_failed_matcher_with_output_cannot_false_pass(self):
        self.executable("awk", 'printf "synthetic-match\\n"\nexit 7\n')
        result = self.run_guard("check-no-pos-magnitude.sh")
        self.assertEqual(result.returncode, 2)
        self.assertIn("self-test command failed", result.stderr)
        self.assertNotIn("PASS:", result.stdout)
        self.assertFalse(list(self.root.rglob("*selftest*")))

    def test_matcher_that_matches_nothing_cannot_false_pass(self):
        self.executable("awk", "exit 0\n")
        result = self.run_guard("check-no-pos-magnitude.sh")
        self.assertEqual(result.returncode, 2)
        self.assertIn("matcher detects nothing", result.stderr)
        self.assertNotIn("PASS:", result.stdout)
        self.assertFalse(list(self.root.rglob("*selftest*")))


if __name__ == "__main__":
    unittest.main(verbosity=2)
