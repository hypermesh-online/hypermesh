#!/usr/bin/env bash
# Phase I.1 — 20-node real-subprocess multi-host harness driver.
#
# Spawns N hypermesh subprocess nodes against the locally-built release
# binary and reports JSON to stdout. Cleans up subprocesses on exit
# (Ctrl-C, error, or natural completion).
#
# Usage:
#   scripts/run-matrix-harness.sh [NODE_COUNT] [SCENARIO]
#
# Where SCENARIO is one of: dns, transfer, loss, all (default: all)
# and NODE_COUNT defaults to 20.
#
# Prereqs:
#   - cargo build --release -p blockmatrix --bin hypermesh --features
#     caesar,intelligence
#   - GLIBC matches the local environment (do NOT use musl for local
#     loopback runs; use musl only for trust.hypermesh.online deployment).
#
# The harness uses these conventions:
#   - Data dirs under /tmp/hm-harness/node-N/
#   - STOQ ports 19000 + N (19000..19019 for 20-node default)
#   - IPC sockets at /tmp/hm-harness/node-N/ctl.sock
#   - Node 0 is the bootstrap reflector (other nodes connect to it via
#     `--bootstrap [::1]:19000`).
#
# This script is a thin orchestrator. The real per-scenario assertions
# live in `blockmatrix/tests/i1_multihost_harness.rs` (in-process) and
# in the JSON report this script emits (subprocess).

set -euo pipefail

NODE_COUNT="${1:-20}"
SCENARIO="${2:-all}"
ROOT="${HM_HARNESS_ROOT:-/tmp/hm-harness}"
BIN="${HM_HARNESS_BIN:-./target/release/hypermesh}"
PORT_BASE="${HM_HARNESS_PORT_BASE:-19000}"
WAIT_SECONDS="${HM_HARNESS_WAIT_SECONDS:-15}"

if [[ ! -x "$BIN" ]]; then
    echo "{\"status\":\"error\",\"reason\":\"binary not found\",\"path\":\"$BIN\",\"hint\":\"cargo build --release -p blockmatrix --bin hypermesh --features caesar,intelligence\"}"
    exit 1
fi

# Clean and recreate harness root
rm -rf "$ROOT"
mkdir -p "$ROOT"

# Track PIDs for cleanup
PIDS=()

cleanup() {
    local rc=$?
    if [[ ${#PIDS[@]} -gt 0 ]]; then
        for pid in "${PIDS[@]}"; do
            kill -TERM "$pid" 2>/dev/null || true
        done
        # Give them a moment to exit gracefully
        sleep 1
        for pid in "${PIDS[@]}"; do
            kill -KILL "$pid" 2>/dev/null || true
        done
    fi
    return $rc
}
trap cleanup EXIT INT TERM

start_node() {
    local idx=$1
    local port=$((PORT_BASE + idx))
    local data_dir="$ROOT/node-$idx"
    local sock="$data_dir/ctl.sock"
    local log="$data_dir/node.log"

    mkdir -p "$data_dir"

    # Node 0 is the bootstrap reflector; subsequent nodes bootstrap to it.
    local extra_args=()
    if [[ $idx -eq 0 ]]; then
        extra_args+=(--reflector)
    else
        extra_args+=(--bootstrap "[::1]:${PORT_BASE}")
    fi

    # IPC socket env override per node
    HYPERMESH_SOCK="$sock" \
        "$BIN" \
        --name "node-$idx" \
        --data-dir "$data_dir" \
        --stoq-port "$port" \
        --network-id "harness-i1" \
        "${extra_args[@]}" \
        connect public --foreground \
        > "$log" 2>&1 &

    PIDS+=($!)
}

# Spawn nodes
for i in $(seq 0 $((NODE_COUNT - 1))); do
    start_node "$i"
done

# Allow nodes to bootstrap and discover peers
sleep "$WAIT_SECONDS"

# Build the JSON report. The real per-scenario assertions are
# expected to be driven by an external test runner (see
# blockmatrix/tests/i1_multihost_harness.rs); this script's job is
# orchestration + a minimal liveness report.
ALIVE=0
DEAD=0
for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
        ALIVE=$((ALIVE + 1))
    else
        DEAD=$((DEAD + 1))
    fi
done

cat <<EOF
{
  "status": "complete",
  "node_count": $NODE_COUNT,
  "scenario": "$SCENARIO",
  "port_base": $PORT_BASE,
  "data_root": "$ROOT",
  "alive": $ALIVE,
  "dead": $DEAD,
  "wait_seconds": $WAIT_SECONDS,
  "note": "Per-scenario assertions are driven by blockmatrix/tests/i1_multihost_harness.rs"
}
EOF

# cleanup runs on exit
