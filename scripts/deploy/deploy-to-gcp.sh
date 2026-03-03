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
for bin in gateway trustchain_ca hypermesh catalog-server engauge-server caesar; do
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
echo "Uploading binaries..."
$SCP "$RELEASE_DIR/gateway" "$RELEASE_DIR/trustchain_ca" "$RELEASE_DIR/hypermesh" "$RELEASE_DIR/catalog-server" "$RELEASE_DIR/engauge-server" "$RELEASE_DIR/caesar" \
    "$REMOTE_USER@$HOST:/tmp/"

$SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
set -euo pipefail
for bin in gateway trustchain_ca hypermesh catalog-server engauge-server caesar; do
    sudo install -m 755 "/tmp/$bin" "/usr/local/bin/$bin"
    rm -f "/tmp/$bin"
done
echo "Binaries installed."
REMOTE

# 4. Upload configs
echo "Uploading configs..."
$SCP "$REPO_ROOT/gateway/config/gateway.toml" "$REMOTE_USER@$HOST:/tmp/gateway.toml"
$SCP "$REPO_ROOT/trustchain/config/production.toml" "$REMOTE_USER@$HOST:/tmp/trustchain.toml"
$SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
sudo install -m 644 /tmp/gateway.toml /etc/hypermesh/gateway.toml
sudo install -m 644 /tmp/trustchain.toml /etc/hypermesh/trustchain.toml
rm -f /tmp/gateway.toml /tmp/trustchain.toml
REMOTE

# 5. Upload systemd units
echo "Uploading systemd units..."
for unit in gateway.service trustchain.service blockmatrix.service catalog.service engauge.service caesar.service; do
    if [ -f "$REPO_ROOT/systemd/$unit" ]; then
        $SCP "$REPO_ROOT/systemd/$unit" "$REMOTE_USER@$HOST:/tmp/$unit"
    fi
done
$SSH "$REMOTE_USER@$HOST" bash -s <<'REMOTE'
set -euo pipefail
for unit in gateway.service trustchain.service blockmatrix.service catalog.service engauge.service caesar.service; do
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
echo "  1. Enable services: ssh $REMOTE_USER@$HOST 'sudo systemctl enable --now trustchain blockmatrix caesar catalog engauge gateway'"
echo "  2. Check status:    ssh $REMOTE_USER@$HOST 'sudo systemctl status trustchain blockmatrix caesar catalog engauge gateway'"
echo "  3. Verify:          curl -k https://$HOST:8443/health"
