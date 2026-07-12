#!/bin/sh
# Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
#
# HyperMesh single-node installer.
#
# One entrypoint that:
#   (a) builds OR reuses the musl static-pie `hypermesh` binary,
#   (b) generates the device identity if absent (FALCON-1024 + Kyber-1024),
#   (c) lets the Substrate self-assign the node's fd48:4d00::/32 address (no DHCP),
#   (d) starts the daemon.
#
# POSIX sh, `set -eu`, idempotent: re-running skips already-satisfied steps. It
# does NOT need to run to completion in this environment — it must be correct and
# readable. All heavy lifting delegates to the `hypermesh` binary (bin of the
# `blockmatrix` crate); identity creation and address self-assignment happen
# inside `hypermesh connect`, not here.

set -eu
# Enable `pipefail` when the shell supports it (bash/ksh/POSIX.1-2024 dash).
# Guarded so plain `sh` without pipefail still runs under `set -eu`.
( set -o pipefail ) 2>/dev/null && set -o pipefail || true

# --- Configuration (override via environment) --------------------------------

# Rust target for the deploy artifact. The remote (Debian 12, glibc 2.36) needs a
# musl static-pie build; NEVER a glibc build (it needs GLIBC_2.39+ symbols the
# remote lacks). See core/CLAUDE.md "Build & Deployment".
TARGET="${HYPERMESH_TARGET:-x86_64-unknown-linux-musl}"

# Where the installed binary lands and where node state lives.
INSTALL_DIR="${HYPERMESH_INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="hypermesh"
INSTALL_PATH="${INSTALL_DIR}/${BINARY_NAME}"
DATA_DIR="${HYPERMESH_DATA_DIR:-${HOME}/.blockmatrix}"

# Runtime parameters.
PRIVACY="${HYPERMESH_PRIVACY:-public}"
STOQ_PORT="${HYPERMESH_STOQ_PORT:-9292}"
NETWORK_ID="${HYPERMESH_NETWORK_ID:-trustnet}"

# Optional prebuilt binary: if set and present, we install it instead of building.
PREBUILT_BINARY="${HYPERMESH_PREBUILT_BINARY:-}"

# Repository root: this script lives in scripts/install/, so root is two up.
SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH='' cd -- "${SCRIPT_DIR}/../.." && pwd)"

# --- Helpers -----------------------------------------------------------------

log()  { printf '[hypermesh-install] %s\n' "$*"; }
die()  { printf '[hypermesh-install] ERROR: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

# --- Step 1: obtain the musl static-pie binary -------------------------------

install_binary() {
    if [ -n "${PREBUILT_BINARY}" ]; then
        [ -f "${PREBUILT_BINARY}" ] || die "HYPERMESH_PREBUILT_BINARY set but not found: ${PREBUILT_BINARY}"
        log "Installing prebuilt binary from ${PREBUILT_BINARY}"
        _stage_binary "${PREBUILT_BINARY}"
        return
    fi

    log "Building ${BINARY_NAME} for ${TARGET} (musl static-pie release)"
    have cargo || die "cargo not found; install the Rust toolchain first"

    # Ensure the musl target is available (idempotent).
    if have rustup; then
        rustup target add "${TARGET}" >/dev/null 2>&1 || true
    fi

    (
        cd "${REPO_ROOT}"
        cargo build --release --target "${TARGET}" -p blockmatrix --bin "${BINARY_NAME}"
    ) || die "cargo build failed"

    built="${REPO_ROOT}/target/${TARGET}/release/${BINARY_NAME}"
    [ -f "${built}" ] || die "expected build artifact missing: ${built}"
    _stage_binary "${built}"
}

# Atomically install a binary at INSTALL_PATH (stage to sibling temp, then rename).
_stage_binary() {
    src="$1"
    mkdir -p "${INSTALL_DIR}" || die "cannot create ${INSTALL_DIR}"
    tmp="${INSTALL_PATH}.staging.$$"
    cp "${src}" "${tmp}" || die "cannot copy binary to ${tmp}"
    chmod 0755 "${tmp}" || die "cannot chmod ${tmp}"
    mv -f "${tmp}" "${INSTALL_PATH}" || { rm -f "${tmp}"; die "cannot install to ${INSTALL_PATH}"; }
    log "Installed ${BINARY_NAME} -> ${INSTALL_PATH}"
}

# --- Step 2 + 3: identity + address ------------------------------------------
#
# Identity generation (FALCON-1024 + Kyber-1024 DER) and fd48:4d00::/32 address
# self-assignment both happen INSIDE the binary on first `connect`:
#   - `FalconIdentity::load_or_create(<data_dir>/<node>/identity)` mints keys if
#     absent (idempotent: existing keys are loaded, never overwritten).
#   - `base::Substrate::active_interface()` selects the carrier-aware interface and
#     `base::derive_address(node_id)` computes the address deterministically — no
#     DHCP, no lease. The address is recomputed identically every boot.
# We only need to ensure the data dir exists; the binary owns the rest.

prepare_state() {
    # Expand a leading ~ in DATA_DIR without invoking a subshell eval.
    case "${DATA_DIR}" in
        "~"/*) DATA_DIR="${HOME}/${DATA_DIR#~/}" ;;
        "~")   DATA_DIR="${HOME}" ;;
    esac
    mkdir -p "${DATA_DIR}" || die "cannot create data dir ${DATA_DIR}"
    log "Node state directory: ${DATA_DIR}"
    log "Identity + fd48:4d00::/32 address are self-provisioned by the daemon on first connect"
}

# --- Step 4: start the daemon ------------------------------------------------

start_daemon() {
    [ -x "${INSTALL_PATH}" ] || die "binary not executable: ${INSTALL_PATH}"
    log "Starting daemon: privacy=${PRIVACY} stoq_port=${STOQ_PORT} network_id=${NETWORK_ID}"

    # `connect` starts the daemon, creating identity if absent and letting the
    # Substrate self-assign the address. It backgrounds itself unless --foreground.
    "${INSTALL_PATH}" \
        --data-dir "${DATA_DIR}" \
        --stoq-port "${STOQ_PORT}" \
        --network-id "${NETWORK_ID}" \
        connect "${PRIVACY}"

    log "Daemon start requested. Check status with: ${BINARY_NAME} status"
}

# --- Main --------------------------------------------------------------------

main() {
    log "HyperMesh installer starting (target=${TARGET})"
    install_binary
    prepare_state
    start_daemon
    log "Done."
}

main "$@"
