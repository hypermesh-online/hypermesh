#!/bin/sh
# Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
# Local digest checking establishes integrity, not authenticated release provenance.
# Linux/GNU utilities are required; shell syntax alone does not imply portability.
set -eu

TARGET=${HYPERMESH_TARGET:-x86_64-unknown-linux-musl}
INSTALL_DIR=${HYPERMESH_INSTALL_DIR:-/usr/local/bin}
DATA_DIR=${HYPERMESH_DATA_DIR:-${HOME}/.blockmatrix}
case "$DATA_DIR" in
    '~') DATA_DIR=$HOME ;;
    '~/'*) DATA_DIR=$HOME/${DATA_DIR#\~/} ;;
esac
INSTALL_PATH=$INSTALL_DIR/hypermesh
PREBUILT_BINARY=${HYPERMESH_PREBUILT_BINARY:-}
EXPECTED_SHA256=${HYPERMESH_PREBUILT_SHA256:-}
PRIVACY=${HYPERMESH_PRIVACY-private}
OFFLINE=${HYPERMESH_OFFLINE-0}
NO_START=${HYPERMESH_NO_START-0}
STOQ_PORT=${HYPERMESH_STOQ_PORT:-9292}
NETWORK_ID=${HYPERMESH_NETWORK_ID:-trustnet}
STAGE=
BACKUP=
COMMITTED=0
SUCCESS=0
HAD_BINARY=0

log() { printf '[hypermesh-install] %s\n' "$*"; }
die() { printf '[hypermesh-install] ERROR: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

rollback() {
    if [ "$HAD_BINARY" = 1 ]; then
        mv -fT -- "$BACKUP" "$INSTALL_PATH" || {
            printf 'Rollback failed; prior binary retained at %s\n' "$BACKUP" >&2
            return 1
        }
        BACKUP=
    else
        rm -f -- "$INSTALL_PATH" || return 1
    fi
}

cleanup() {
    status=$?
    trap - 0
    trap '' HUP INT TERM
    # Retain the old binary until every other fallible cleanup has completed.
    if [ -n "$STAGE" ]; then
        if ! rm -rf -- "$STAGE"; then
            SUCCESS=0
            status=1
        fi
    fi
    if [ "$SUCCESS" = 1 ] && [ -n "$BACKUP" ]; then
        if rm -f -- "$BACKUP"; then
            BACKUP=
        else
            SUCCESS=0
            status=1
        fi
    fi
    if [ "$COMMITTED" = 1 ] && [ "$SUCCESS" = 0 ]; then
        rollback || exit 1
        if [ -n "$STAGE" ]; then
            rm -rf -- "$STAGE" || status=1
        fi
    fi
    if [ -n "$BACKUP" ]; then
        rm -f -- "$BACKUP" || status=1
    fi
    exit "$status"
}
trap cleanup 0
trap 'exit 1' HUP INT TERM

preflight() {
    case "$OFFLINE:$NO_START" in 0:0|0:1|1:0|1:1) ;; *) die 'flags must be 0 or 1';; esac
    case "$PRIVACY" in private|public|anonymous) ;; *) die 'invalid privacy';; esac
    case "$(mv --help 2>/dev/null)" in
        *--no-target-directory*) ;;
        *) die 'GNU mv with --no-target-directory (-T) is required' ;;
    esac
    if [ "$OFFLINE" = 1 ]; then
        [ "$NO_START" = 1 ] && [ "$PRIVACY" = private ] || die 'offline requires private and no-start'
        [ -n "${HYPERMESH_INSTALL_DIR:-}" ] && [ -n "${HYPERMESH_DATA_DIR:-}" ] || die 'offline requires explicit roots'
        [ -n "$PREBUILT_BINARY" ] || die 'offline requires a prebuilt artifact'
    fi
    if [ -n "$PREBUILT_BINARY" ]; then
        [ ${#EXPECTED_SHA256} -eq 64 ] || die 'prebuilt requires a 64-hex SHA256'
        case "$EXPECTED_SHA256" in *[!0123456789abcdefABCDEF]*) die 'invalid SHA256';; esac
        [ -f "$PREBUILT_BINARY" ] && [ -r "$PREBUILT_BINARY" ] || die 'artifact not readable'
        have sha256sum || die 'sha256sum required'
    elif [ -n "$EXPECTED_SHA256" ]; then
        die 'digest requires a prebuilt artifact'
    fi
    if [ "$OFFLINE" = 1 ]; then
        case "$INSTALL_DIR" in /*) ;; *) die 'offline install root must be absolute';; esac
        case "$DATA_DIR" in /*) ;; *) die 'offline data root must be absolute';; esac
    fi
    [ ! -L "$INSTALL_DIR" ] && [ ! -L "$DATA_DIR" ] || die 'roots must not be symlinks'
    [ ! -L "$INSTALL_PATH" ] || die 'installed binary must not be a symlink'
    [ ! -e "$INSTALL_PATH" ] || [ -f "$INSTALL_PATH" ] || die 'invalid installed binary'
}

prepare_state() {
    mkdir -p -- "$DATA_DIR" || die 'cannot prepare data directory'
    [ -d "$DATA_DIR" ] && [ -w "$DATA_DIR" ] || die 'data directory not writable'
    [ "$(stat -c %u -- "$DATA_DIR")" = "$(id -u)" ] || die 'data directory must be caller-owned'
    mkdir -p -- "$INSTALL_DIR" || die 'cannot prepare install directory'
    [ -d "$INSTALL_DIR" ] && [ -w "$INSTALL_DIR" ] || die 'install directory not writable'
}

obtain_binary() {
    if [ -n "$PREBUILT_BINARY" ]; then
        SOURCE=$PREBUILT_BINARY
        return
    fi
    have cargo || die 'cargo not found; install Rust first'
    SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
    REPO_ROOT=$(CDPATH='' cd -- "$SCRIPT_DIR/../.." && pwd)
    if have rustup; then
        rustup target add "$TARGET" >/dev/null 2>&1 || true
    fi
    (cd "$REPO_ROOT" && cargo build --release --target "$TARGET" -p blockmatrix --bin hypermesh) || die 'cargo build failed'
    SOURCE=$REPO_ROOT/target/$TARGET/release/hypermesh
    [ -f "$SOURCE" ] || die 'build artifact missing'
}

stage_binary() {
    STAGE=$(mktemp -d "$INSTALL_DIR/.hypermesh-install.XXXXXXXXXX") || die 'cannot create private staging directory'
    chmod 0700 -- "$STAGE" || die 'cannot protect staging directory'
    cp -- "$SOURCE" "$STAGE/new" || die 'cannot stage binary'
    if [ -n "$PREBUILT_BINARY" ]; then
        actual=$(sha256sum < "$STAGE/new") || die 'cannot hash staged binary'
        actual=${actual%% *}
        expected=$(printf '%s' "$EXPECTED_SHA256" | tr ABCDEF abcdef)
        [ "$actual" = "$expected" ] || die 'staged binary SHA256 mismatch'
    fi
    chmod 0755 -- "$STAGE/new" || die 'cannot chmod staged binary'
    if [ -f "$INSTALL_PATH" ]; then
        BACKUP=$(mktemp "$INSTALL_DIR/.hypermesh-previous.XXXXXXXXXX") || die 'cannot reserve backup'
        cp -p -- "$INSTALL_PATH" "$BACKUP" || die 'cannot preserve previous binary'
        HAD_BINARY=1
    fi
}

start_daemon() {
    "$INSTALL_PATH" --data-dir "$DATA_DIR" --stoq-port "$STOQ_PORT" \
        --network-id "$NETWORK_ID" connect "$PRIVACY" || die 'daemon start failed'
}

main() {
    preflight
    prepare_state
    obtain_binary
    stage_binary
    # Record intent first so a delivered signal immediately after rename rolls back.
    COMMITTED=1
    mv -fT -- "$STAGE/new" "$INSTALL_PATH" || die 'cannot atomically install binary'
    if [ "$NO_START" = 0 ]; then
        start_daemon
    fi
    # No fallible operations after success: cleanup is handled by the EXIT trap.
    SUCCESS=1
}
main "$@"
