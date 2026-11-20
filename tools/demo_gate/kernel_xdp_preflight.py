#!/usr/bin/env python3
"""Read-only kernel prerequisite observation; NEVER approves a demo."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import os
import stat
import socket
import subprocess
import time


class Unobserved(ValueError):
    """No independently observed kernel fact."""


TRUSTED_BPFTOOL_PATHS = ("/usr/sbin/bpftool", "/usr/bin/bpftool", "/sbin/bpftool", "/bin/bpftool")


def require_trusted(path):
    resolved = Path(path).resolve(strict=True)
    for entry in (resolved, *resolved.parents):
        state = entry.stat()
        if state.st_uid != 0 or state.st_mode & 0o022:
            raise Unobserved("bpftool path is not root-owned and non-writable")
    if not stat.S_ISREG(resolved.stat().st_mode) or not os.access(resolved, os.X_OK):
        raise Unobserved("bpftool is not a trusted executable file")
    return str(resolved)


def trusted_bpftool():
    for path in TRUSTED_BPFTOOL_PATHS:
        if Path(path).exists():
            return require_trusted(path)
    raise Unobserved("trusted system bpftool unavailable; PATH is not consulted")


def kernel_query(arguments):
    binary = trusted_bpftool()
    command = [binary, "-j", *arguments]
    started = time.time_ns()
    result = subprocess.run(command, capture_output=True, timeout=5, check=False)
    return {"argv": command, "started_ns": started, "ended_ns": time.time_ns(),
            "exit": result.returncode,
            "tool_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
            "stdout": result.stdout.decode("utf-8", errors="strict"),
            "stderr": result.stderr.decode("utf-8", errors="strict")}


def checked_json(arguments, query, observations):
    record = query(arguments)
    observations.append(record)
    if record["exit"] != 0:
        raise Unobserved("kernel inspection refused or failed; no fallback")
    return json.loads(record["stdout"])


def xdp_inventory(document, interface, ifindex):
    if not isinstance(document, list):
        raise Unobserved("unrecognized bpftool net schema")
    entries = []
    for namespace in document:
        if not isinstance(namespace, dict):
            raise Unobserved("unrecognized namespace record")
        entries.extend(namespace.get("xdp", []))
    selected = [row for row in entries if row.get("devname") == interface]
    inventory = []
    for row in selected:
        if "multi_attachments" in row:
            raise Unobserved("multi_attachments schema unsupported; observation refused")
        if type(row.get("ifindex")) is not int or row["ifindex"] != ifindex:
            raise Unobserved("kernel interface index does not match local interface")
        mode, program_id = row.get("mode"), row.get("id")
        if mode not in ("generic", "driver", "offload"):
            raise Unobserved("missing or unsupported attachment mode")
        if type(program_id) is not int or program_id <= 0:
            raise Unobserved("missing kernel program ID")
        inventory.append((mode, program_id))
    if not inventory:
        raise Unobserved("no kernel-observed XDP attachment on selected interface")
    return sorted(set(inventory))


def program_metadata(document, program_id):
    rows = document if isinstance(document, list) else [document]
    matches = [row for row in rows if isinstance(row, dict) and row.get("id") == program_id]
    if len(matches) != 1:
        raise Unobserved("attached program absent or ambiguous in kernel inventory")
    row = matches[0]
    if row.get("type") != "xdp" or not re.fullmatch(r"[0-9a-f]{16}", row.get("tag", "")):
        raise Unobserved("attached ID lacks valid kernel XDP program metadata")
    return {key: row.get(key) for key in ("id", "type", "tag", "name", "map_ids")}


def observe(interface, query=kernel_query, resolve=socket.if_nametoindex):
    report = {"profile": "hypermesh-kernel-prerequisite-v1", "interface": interface,
              "kernel_attachment": "unobserved", "demo_verified": False,
              "observations": [], "missing_demo_evidence": [
                  "approved source/artifact binding to kernel program",
                  "kernel counter deltas causally linked to exact transfer and controls",
                  "two independent production nodes and byte-identical Catalog asset",
                  "authentic action/asset/network-scoped fresh proof and negative controls"]}
    try:
        net = ["net", "show", "dev", interface]
        local_index = resolve(interface)
        first = xdp_inventory(checked_json(net, query, report["observations"]), interface, local_index)
        programs = [program_metadata(checked_json(["prog", "show", "id", str(pid)],
                    query, report["observations"]), pid) for pid in sorted({pid for _, pid in first})]
        last = xdp_inventory(checked_json(net, query, report["observations"]), interface, local_index)
        if first != last or resolve(interface) != local_index:
            raise Unobserved("interface index or attachment (mode, ID) changed during observation")
        report.update(kernel_attachment="observed", ifindex=local_index, attachments=first, programs=programs)
    except (Unobserved, OSError, ValueError, KeyError, TypeError, AttributeError,
            subprocess.TimeoutExpired) as error:
        report["reason"] = str(error)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interface", required=True, help="existing local interface")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.:-]{1,15}", args.interface):
        parser.error("invalid local interface name")
    try:
        socket.if_nametoindex(args.interface)
    except OSError:
        parser.error("local interface does not exist")
    report = observe(args.interface)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["kernel_attachment"] == "observed" else 2


if __name__ == "__main__":
    raise SystemExit(main())
