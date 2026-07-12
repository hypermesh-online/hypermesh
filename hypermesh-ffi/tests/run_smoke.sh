#!/usr/bin/env bash
# Copyright 2026 Hypermesh Foundation. All rights reserved.
# Licensed under the Business Source License 1.1.
#
# Build the C smoke test against the built HyperMesh FFI library and run it.
# Tries the staticlib first (as requested); falls back to the cdylib if the
# static link cannot resolve the full native dependency graph.
#
# Usage: hypermesh-ffi/tests/run_smoke.sh [target-dir]

set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_DIR="$(cd "${CRATE_DIR}/.." && pwd)"
TARGET_DIR="${1:-${WORKSPACE_DIR}/target/debug}"

HEADER_DIR="${CRATE_DIR}/include"
SRC="${CRATE_DIR}/tests/smoke.c"
STATIC_LIB="${TARGET_DIR}/libhypermesh_ffi.a"
DYN_LIB="${TARGET_DIR}/libhypermesh_ffi.so"
OUT="${TARGET_DIR}/hypermesh_ffi_smoke"

echo "crate:      ${CRATE_DIR}"
echo "target:     ${TARGET_DIR}"
echo "header:     ${HEADER_DIR}/hypermesh.h"

# System libraries a Rust staticlib pulls in transitively.
SYS_LIBS="-lpthread -ldl -lm -lrt"

link_static() {
    echo ">>> Attempting static link against ${STATIC_LIB}"
    cc -std=c11 -Wall -Wextra -O2 \
        -I"${HEADER_DIR}" \
        "${SRC}" \
        "${STATIC_LIB}" \
        ${SYS_LIBS} \
        -o "${OUT}"
}

link_dynamic() {
    echo ">>> Falling back to dynamic link against ${DYN_LIB}"
    cc -std=c11 -Wall -Wextra -O2 \
        -I"${HEADER_DIR}" \
        "${SRC}" \
        -L"${TARGET_DIR}" -lhypermesh_ffi \
        -o "${OUT}"
}

MODE="static"
if [ -f "${STATIC_LIB}" ] && link_static 2>/tmp/hm_ffi_static_link.log; then
    MODE="static"
else
    echo ">>> Static link failed or staticlib missing; see /tmp/hm_ffi_static_link.log"
    link_dynamic
    MODE="dynamic"
fi

echo ">>> Linked (${MODE}). Running smoke test:"
echo "-----------------------------------------------------------"
if [ "${MODE}" = "dynamic" ]; then
    LD_LIBRARY_PATH="${TARGET_DIR}:${LD_LIBRARY_PATH:-}" "${OUT}"
else
    "${OUT}"
fi
RC=$?
echo "-----------------------------------------------------------"
echo ">>> smoke test exit code: ${RC} (link mode: ${MODE})"
exit ${RC}
