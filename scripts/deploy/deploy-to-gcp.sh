#!/usr/bin/env bash
# Deploy HyperMesh binaries and configs to a remote host.
# Usage: ./scripts/deploy/deploy-to-gcp.sh <host> [--user <user>] [--key <ssh-key>]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOST="${1:?Usage: deploy-to-gcp.sh <host> [--user <user>] [--key <ssh-key>]}"
shift

# Parse optional args
REMOTE_USER="persist"
SSH_KEY="$HOME/.ssh/hypermesh_deploy"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --user)  REMOTE_USER="$2"; shift 2 ;;
        --key)   SSH_KEY="$2"; shift 2 ;;
        *)       echo "Unknown option: $1"; exit 1 ;;
    esac
done

SSH_OPTS="-o StrictHostKeyChecking=accept-new"
if [ -f "$SSH_KEY" ]; then
    SSH_OPTS="$SSH_OPTS -i $SSH_KEY"
fi
SSH="ssh $SSH_OPTS"
SCP="scp $SSH_OPTS"

# Prefer static musl binaries, fall back to glibc
MUSL_DIR="$REPO_ROOT/target/x86_64-unknown-linux-musl/release"
GLIBC_DIR="$REPO_ROOT/target/release"
if [ -f "$MUSL_DIR/gateway" ]; then
    RELEASE_DIR="$MUSL_DIR"
    echo "Using static musl binaries (portable)."
else
    RELEASE_DIR="$GLIBC_DIR"
    echo "Using glibc binaries (ensure target has compatible glibc)."
fi

echo "=== Deploying HyperMesh to $HOST (user: $REMOTE_USER) ==="

# 1. Ensure binaries exist
# engauge-server is intentionally excluded: disabled on the remote and it needs a
# `server` cargo feature that is not part of the deploy build.
for bin in gateway trustchain-http3-server trustchain-stoq-server hypermesh catalog-server caesar; do
    if [ ! -f "$RELEASE_DIR/$bin" ]; then
        echo "ERROR: $RELEASE_DIR/$bin not found. Run build-release.sh first."
        exit 1
    fi
done

# 2. Create remote directories
echo "Creating remote directories..."
$SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
set -euo pipefail
sudo id -u hypermesh &>/dev/null || sudo useradd -r -s /usr/sbin/nologin -d /var/lib/hypermesh hypermesh
sudo mkdir -p /var/lib/hypermesh/{blockmatrix,trustchain,gateway,catalog,engauge,caesar}
sudo mkdir -p /var/log/hypermesh
sudo mkdir -p /etc/hypermesh/certs
sudo chown -R hypermesh:hypermesh /var/lib/hypermesh /var/log/hypermesh /etc/hypermesh
REMOTE

# 3. Upload binaries (to /tmp first, then sudo mv)
# F9: trustchain_ca (dev tool using StateProof::new_for_testing) is NOT deployed.
# Production CAs are trustchain-http3-server (clearnet) + trustchain-stoq-server
# (node-to-node), both free of test proofs.
echo "Uploading binaries..."
$SCP "$RELEASE_DIR/gateway" "$RELEASE_DIR/trustchain-http3-server" "$RELEASE_DIR/trustchain-stoq-server" "$RELEASE_DIR/hypermesh" "$RELEASE_DIR/catalog-server" "$RELEASE_DIR/caesar" \
    "$REMOTE_USER@$HOST:/tmp/"

# 3b. Upload the compiled XDP kernel object so the node's eBPF gate can attach
# (blockmatrix.service grants CAP_BPF+CAP_NET_ADMIN by default).
if [ -f "$REPO_ROOT/target/bpf/hypermesh_xdp.o" ]; then
    $SCP "$REPO_ROOT/target/bpf/hypermesh_xdp.o" "$REMOTE_USER@$HOST:/tmp/hypermesh_xdp.o"
    $SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
set -euo pipefail
# Reference copy under /usr/local/lib/hypermesh (documentation / manual attach).
sudo install -d -m 755 /usr/local/lib/hypermesh
sudo install -m 644 /tmp/hypermesh_xdp.o /usr/local/lib/hypermesh/hypermesh_xdp.o
# Loader copy: the node (blockmatrix.service, WorkingDirectory=/var/lib/hypermesh/blockmatrix)
# searches for "target/bpf/hypermesh_xdp.o" relative to its CWD. Install there so the
# XDP program is actually found and the kernel gate attaches instead of degrading to
# userspace-only. Owned by hypermesh:hypermesh to match the service user.
sudo install -d -o hypermesh -g hypermesh -m 755 /var/lib/hypermesh/blockmatrix/target/bpf
sudo install -o hypermesh -g hypermesh -m 644 /tmp/hypermesh_xdp.o /var/lib/hypermesh/blockmatrix/target/bpf/hypermesh_xdp.o
rm -f /tmp/hypermesh_xdp.o
echo "XDP kernel object installed (reference + loader search path)."
REMOTE
else
    echo "WARNING: target/bpf/hypermesh_xdp.o not found — kernel eBPF gate will fall back to userspace-only."
fi

$SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
set -euo pipefail
for bin in gateway trustchain-http3-server trustchain-stoq-server hypermesh catalog-server caesar; do
    sudo install -m 755 "/tmp/$bin" "/usr/local/bin/$bin"
    rm -f "/tmp/$bin"
done
echo "Binaries installed."
REMOTE

# 4. Upload configs
# The two per-service CA configs (trustchain-http3.toml / trustchain-stoq.toml)
# are schema-valid for TrustChainConfig::load() and carry the production [api]
# bar (F9). The systemd units point TRUSTCHAIN_CONFIG at these files.
echo "Uploading configs..."
$SCP "$REPO_ROOT/gateway/config/gateway.toml" "$REMOTE_USER@$HOST:/tmp/gateway.toml"
$SCP "$REPO_ROOT/trustchain/config/trustchain-http3.toml" "$REMOTE_USER@$HOST:/tmp/trustchain-http3.toml"
$SCP "$REPO_ROOT/trustchain/config/trustchain-stoq.toml" "$REMOTE_USER@$HOST:/tmp/trustchain-stoq.toml"
$SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
sudo install -m 644 /tmp/gateway.toml /etc/hypermesh/gateway.toml
sudo install -m 644 /tmp/trustchain-http3.toml /etc/hypermesh/trustchain-http3.toml
sudo install -m 644 /tmp/trustchain-stoq.toml /etc/hypermesh/trustchain-stoq.toml
rm -f /tmp/gateway.toml /tmp/trustchain-http3.toml /tmp/trustchain-stoq.toml
REMOTE

# 5. Upload systemd units
# engauge.service is omitted: the engauge-server binary is not deployed and the
# service stays disabled on the remote.
echo "Uploading systemd units..."
UNITS="gateway.service trustchain-http3.service trustchain-stoq.service blockmatrix.service catalog.service caesar.service"
for unit in $UNITS; do
    if [ -f "$REPO_ROOT/systemd/$unit" ]; then
        $SCP "$REPO_ROOT/systemd/$unit" "$REMOTE_USER@$HOST:/tmp/$unit"
    fi
done
$SSH "$REMOTE_USER@$HOST" UNITS="$UNITS" bash -s <<'REMOTE'
set -euo pipefail
# F9: retire the old single trustchain.service (ran the dev trustchain_ca).
if systemctl list-unit-files trustchain.service >/dev/null 2>&1; then
    sudo systemctl disable --now trustchain.service 2>/dev/null || true
    sudo rm -f /etc/systemd/system/trustchain.service
fi
for unit in $UNITS; do
    if [ -f "/tmp/$unit" ]; then
        sudo install -m 644 "/tmp/$unit" "/etc/systemd/system/$unit"
        rm -f "/tmp/$unit"
    fi
done
sudo systemctl daemon-reload
echo "Systemd units installed."
REMOTE

echo ""
echo "=== Deployment complete ==="
echo "Next steps:"
echo "  1. Enable services: ssh $REMOTE_USER@$HOST 'sudo systemctl enable --now trustchain-stoq trustchain-http3 blockmatrix gateway caesar catalog'"
echo "  2. Check status:    ssh $REMOTE_USER@$HOST 'sudo systemctl status trustchain-stoq trustchain-http3 blockmatrix gateway caesar catalog'"
echo "  3. Verify:          curl -k https://$HOST:8443/health"
