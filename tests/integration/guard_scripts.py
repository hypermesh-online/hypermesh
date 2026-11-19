#!/usr/bin/env python3
"""Bounded script integration fixtures; run with TMPDIR on caller-owned SSD storage.

TMPDIR=/usr/lib/hypermesh/verification/... python3 tests/integration/guard_scripts.py
TEST_GUARD_SOURCE optionally selects another snapshot for baseline regression runs.
No real large files, local hooks, Git operations or live disk-pressure changes.
"""
import os
from pathlib import Path
import shutil
import shlex
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

    def assert_fatal(self, name, marker):
        result = self.run_guard(name)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertNotIn("PASS:", result.stdout)
        self.assertIn(marker, result.stderr)

    def fail_tool(self, name, selector="", output=""):
        real = shlex.quote(shutil.which(name))
        condition = f'case "$*" in *{shlex.quote(selector)}*)' if selector else ""
        stub = f'{condition}\nprintf %s {shlex.quote(output)}\nexit 9\n'
        if selector:
            stub += f';; *) exec {real} "$@";; esac\n'
        self.executable(name, stub)


class BuildFixture(GuardFixture):
    def setUp(self):
        super().setUp()
        self.env.update(GUARD_SIZE_KB="1048576", GUARD_FREE_GB="200")
        self.executable("du", 'printf "%s\\t%s\\n" "$GUARD_SIZE_KB" "$2"\n')
        self.executable("df", 'if [ "$1" = -k ]; then\n'
                        'printf "Avail\\n%s\\n" "${GUARD_FREE_KB:-$((GUARD_FREE_GB * 1048576))}"\n'
                        'else printf "Avail\\n%sG\\n" "$GUARD_FREE_GB"; fi\n')

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


class BuildExactBudgetTests(BuildFixture):
    def test_per_tree_plus_one_kib_rejected(self):
        self.target()
        self.env["GUARD_SIZE_KB"] = str(80 * 1048576 + 1)
        self.assert_result(self.run_guard("check-build-size.sh"), 1, "per-tree budget")

    def test_aggregate_exact_and_plus_one_kib(self):
        self.target()
        self.target(".claude/worktrees/peer/target")
        self.env["GUARD_SIZE_KB"] = str(70 * 1048576)
        self.assert_result(self.run_guard("check-build-size.sh"), 0, "PASS:")
        self.executable("du", 'case "$2" in ./target) n=73400320;; *) n=73400321;; esac\n'
                        'printf "%s\\t%s\\n" "$n" "$2"\n')
        self.assert_result(self.run_guard("check-build-size.sh"), 1, "over the 140 GiB")

    def test_free_space_exact_and_one_kib_below(self):
        self.env["GUARD_FREE_KB"] = str(60 * 1048576)
        self.assert_result(self.run_guard("check-build-size.sh"), 0, "PASS:")
        self.env["GUARD_FREE_KB"] = str(60 * 1048576 - 1)
        self.assert_result(self.run_guard("check-build-size.sh"), 1, "free")

    def test_target_with_spaces_and_newline(self):
        self.target(".claude/worktrees/a b\nc/target")
        self.assert_result(self.run_guard("check-build-size.sh"), 0, "PASS:")


class BuildOperationalTests(BuildFixture):
    def test_find_failure_rejected_even_with_partial_inventory(self):
        self.target()
        self.executable("find", 'printf "./target\\0"\nexit 9\n')
        self.assert_fatal("check-build-size.sh", "inventory")

    def test_missing_du_measurement_rejected(self):
        self.target()
        self.executable("du", "exit 0\n")
        self.assert_fatal("check-build-size.sh", "measurement")

    def test_du_failure_rejected_despite_numeric_output(self):
        self.target()
        self.executable("du", 'printf "1\\t%s\\n" "$2"\nexit 9\n')
        self.assert_fatal("check-build-size.sh", "du")

    def test_df_failure_rejected_despite_numeric_output(self):
        self.executable("df", 'printf "Avail\\n209715200\\n"\nexit 9\n')
        self.assert_fatal("check-build-size.sh", "df")

    def test_missing_df_measurement_rejected(self):
        self.executable("df", 'printf "Avail\\n"\n')
        self.assert_fatal("check-build-size.sh", "measurement")


class BuildMeasurementTests(BuildFixture):
    def test_malformed_du_values_rejected(self):
        self.target()
        for value in ["-1", "1.2", "bad", "9223372036854775808", "1\n2"]:
            with self.subTest(value=value):
                self.env["GUARD_SIZE_KB"] = value
                self.assert_fatal("check-build-size.sh", "measurement")

    def test_malformed_df_values_rejected(self):
        for value in ["-1", "1G", "x200", "9223372036854775808", "1\n2"]:
            with self.subTest(value=value):
                self.env["GUARD_FREE_KB"] = value
                self.assert_fatal("check-build-size.sh", "measurement")

    def test_du_wrong_path_rejected(self):
        self.target()
        self.executable("du", 'printf "1\\t./other\\n"\n')
        self.assert_fatal("check-build-size.sh", "measurement")

    def test_df_extra_row_rejected(self):
        self.executable("df", 'printf "Avail\\n209715200\\n209715200\\n"\n')
        self.assert_fatal("check-build-size.sh", "measurement")

    def test_total_integer_overflow_rejected(self):
        self.target()
        self.target(".claude/worktrees/peer/target")
        self.env["GUARD_SIZE_KB"] = "9223372036854775807"
        self.assert_fatal("check-build-size.sh", "overflow")


class ScannerOperationalTests(ProofFixture):
    def test_find_failure_rejected_with_partial_inventory(self):
        self.executable("find", 'printf "./orphan.rs\\0"\nexit 9\n')
        self.assert_fatal("check-no-pos-magnitude.sh", "inventory")

    def test_production_awk_failure_rejected(self):
        self.fail_tool("awk", 'printf "%s:%d:%s')
        self.assert_fatal("check-no-pos-magnitude.sh", "scan")

    def test_production_xargs_failure_rejected(self):
        self.fail_tool("xargs")
        self.assert_fatal("check-no-pos-magnitude.sh", "scan")

    def test_duplicate_grep_failure_rejected(self):
        self.fail_tool("grep", "pub struct")
        self.assert_fatal("check-no-pos-magnitude.sh", "duplicate")

    def test_exception_filter_failure_rejected(self):
        self.fail_tool("grep", "chain")
        self.assert_fatal("check-no-pos-magnitude.sh", "scan")

    def test_inventory_of_missing_source_rejected(self):
        self.executable("find", 'printf "./missing.rs\\0"\n')
        self.assert_fatal("check-no-pos-magnitude.sh", "scan")


class ScannerScopeTests(ProofFixture):
    def test_clean_empty_scope_passes(self):
        (self.root / "orphan.rs").unlink()
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "scanning 0 files")

    def test_allowlisted_only_scope_passes(self):
        (self.root / "orphan.rs").unlink()
        p = self.root / "caesar/src/allowed.rs"
        p.parent.mkdir(parents=True)
        p.write_text("fn f() { let stake_amount = 1; }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "scanning 0 files")

    def test_narrow_exception_does_not_hide_other_violations(self):
        p = self.root / "blockmatrix/src/blockchain/chain.rs"
        p.parent.mkdir(parents=True)
        p.write_text("fn f() { if total_size == 0 {} }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "PASS:")
        p.write_text("fn f() { let stake_amount = 1; }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "stake_amount")

    def test_source_filename_spaces_and_newline_still_scanned(self):
        (self.root / "orphan.rs").unlink()
        (self.root / "odd name\nextra.rs").write_text("fn f() { let stake_amount = 1; }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "stake_amount")

    def test_exception_extension_dot_remains_literal(self):
        p = self.root / "blockmatrix/src/blockchain/chainXrs.c"
        p.parent.mkdir(parents=True)
        p.write_text("fn f() { if total_size == 0 {} }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "capacity never gates")

    def test_multiline_filename_cannot_forge_a_full_exception(self):
        p = self.root / "blockmatrix/src/blockchain/chain.rs:1:total_size\norphan.rs"
        p.parent.mkdir(parents=True)
        p.write_text("fn f() { let stake_amount = 1; }\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "stake_amount")

    def test_duplicate_definition_detected(self):
        (self.root / "orphan.rs").write_text("pub struct StakeProof {}\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "duplicate PoS")



class ScannerBatchTests(ProofFixture):
    def test_large_clean_inventory_and_late_duplicate(self):
        for i in range(9500):
            (self.root / (f"{i:05d}-" + "x" * 225 + ".rs")).touch()
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 0, "PASS:")
        (self.root / "late.rs").write_text("pub struct StakeProof {}\n")
        self.assert_result(self.run_guard("check-no-pos-magnitude.sh"), 1, "duplicate PoS")

    def test_failed_duplicate_batch_is_not_hidden_by_clean_batches(self):
        real = shlex.quote(shutil.which("xargs"))
        self.executable("xargs", f'exec {real} -n 1 "$@"\n')
        (self.root / "second.rs").touch()
        self.fail_tool("grep", "orphan.rs")
        self.assert_fatal("check-no-pos-magnitude.sh", "duplicate")


if __name__ == "__main__":
    unittest.main(verbosity=2)
