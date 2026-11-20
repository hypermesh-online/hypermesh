"""Gate refusal unit fixtures, NOT kernel or network evidence."""
import importlib.util
import io
import sys
import json
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location(
    "probe", ROOT / "tools/demo_gate/kernel_xdp_preflight.py")
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


def response(value, code=0):
    return {"exit": code, "stdout": json.dumps(value), "stderr": "fixture only"}


def attached(program_id=7, ifindex=9, mode="generic"):
    return [{"xdp": [{"devname": "demo0", "id": program_id, "ifindex": ifindex, "mode": mode}]}]


class RefusalTests(unittest.TestCase):
    def refused(self, replies):
        report = PROBE.observe("demo0", query=lambda _: replies.pop(0), resolve=lambda _: 9)
        self.assertFalse(report["demo_verified"])
        self.assertEqual(report["kernel_attachment"], "unobserved")

    def test_project_simulation_metrics_not_kernel_attachment(self):
        self.refused([response({"attached": True, "packets_processed": 12345})])

    def test_compiled_or_named_program_without_attachment_refused(self):
        self.refused([response([{"xdp": []}])])

    def test_permission_refusal_not_fallback_pass(self):
        self.refused([response({}, code=1)])

    def test_missing_tool(self):
        with patch.object(PROBE, "trusted_bpftool", side_effect=PROBE.Unobserved("unavailable")):
            report = PROBE.observe("demo0", resolve=lambda _: 9)
        self.assertFalse(report["demo_verified"])
        self.assertIn("unavailable", report["reason"])

    def test_malformed_json(self):
        self.refused([{"exit": 0, "stdout": "not JSON", "stderr": ""}])

    def test_wrong_program_id(self):
        self.refused([response(attached()), response({"id": 8, "type": "xdp"})])

    def test_wrong_program_type(self):
        self.refused([response(attached()), response({"id": 7, "type": "kprobe"})])

    def test_attachment_race(self):
        self.refused([response(attached()), response({"id": 7, "type": "xdp",
                     "tag": "0123456789abcdef"}), response(attached(8))])


class ObservationTests(unittest.TestCase):
    def test_observation_never_marks_demo_verified(self):
        replies = [response(attached()), response({"id": 7, "type": "xdp",
                   "tag": "0123456789abcdef"}), response(attached())]
        calls = []

        def query(arguments):
            calls.append(arguments)
            return replies.pop(0)

        report = PROBE.observe("demo0", query=query, resolve=lambda _: 9)
        self.assertEqual(report["kernel_attachment"], "observed")
        self.assertFalse(report["demo_verified"])
        self.assertEqual(len(report["missing_demo_evidence"]), 4)
        self.assertEqual(calls, [["net", "show", "dev", "demo0"],
                         ["prog", "show", "id", "7"], ["net", "show", "dev", "demo0"]])


class CliFixtureTests(unittest.TestCase):
    def invoke(self, replies):
        observe = PROBE.observe

        def fixture(interface):
            return observe(interface, query=lambda _: replies.pop(0), resolve=lambda _: 9)

        output = io.StringIO()
        with patch.object(sys, "argv", ["probe", "--interface", "demo0"]), patch.object(PROBE.socket, "if_nametoindex", return_value=9), patch.object(PROBE, "observe", side_effect=fixture), patch.object(sys, "stdout", output):
            code = PROBE.main()
        return code, json.loads(output.getvalue())

    def test_cli_observed_fixture_zero_never_demo_pass(self):
        replies = [response(attached()), response({"id": 7, "type": "xdp",
                   "tag": "0123456789abcdef"}), response(attached())]
        code, report = self.invoke(replies)
        self.assertEqual(code, 0)
        self.assertFalse(report["demo_verified"])

    def test_cli_unobserved_fixture_nonzero(self):
        code, report = self.invoke([response([{"xdp": []}])])
        self.assertEqual(code, 2)
        self.assertFalse(report["demo_verified"])


class StableIdentityTests(unittest.TestCase):
    def refused(self, replies):
        return RefusalTests.refused(self, replies)

    def test_kernel_ifindex_disagrees_with_local_interface(self):
        self.refused([response(attached(ifindex=22))])

    def test_mode_change_with_same_program_id_refused(self):
        self.refused([response(attached()), response({"id": 7, "type": "xdp",
                     "tag": "0123456789abcdef"}), response(attached(mode="driver"))])

    def test_ifindex_change_with_same_program_id_refused(self):
        self.refused([response(attached()), response({"id": 7, "type": "xdp",
                     "tag": "0123456789abcdef"}), response(attached(ifindex=22))])

    def test_multi_attachments_explicitly_unsupported(self):
        self.refused([response([{"xdp": [{"devname": "demo0", "ifindex": 9,
                     "multi_attachments": [{"id": 7, "mode": "generic"}]}]}])])

    def test_local_interface_replacement_refused(self):
        replies = [response(attached()), response({"id": 7, "type": "xdp",
                   "tag": "0123456789abcdef"}), response(attached())]
        indices = iter([9, 22])
        report = PROBE.observe("demo0", query=lambda _: replies.pop(0), resolve=lambda _: next(indices))
        self.assertEqual(report["kernel_attachment"], "unobserved")


class ExecutableTrustTests(unittest.TestCase):
    def test_arbitrary_path_is_never_consulted(self):
        with patch.object(PROBE, "TRUSTED_BPFTOOL_PATHS", ()), patch.dict(PROBE.os.environ, {"PATH": "/arbitrary/writable"}):
            with self.assertRaises(PROBE.Unobserved):
                PROBE.trusted_bpftool()

    def test_writable_or_nonroot_path_refused(self):
        from types import SimpleNamespace
        for owner, mode in ((1000, 0o100755), (0, 0o100777)):
            with patch.object(PROBE.Path, "stat", return_value=SimpleNamespace(st_uid=owner, st_mode=mode)):
                with self.assertRaises(PROBE.Unobserved):
                    PROBE.require_trusted("/usr/bin/python3")


if __name__ == "__main__":
    unittest.main()
