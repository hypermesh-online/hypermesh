#!/usr/bin/env bash
# Deploy HyperMesh binaries and configs to a remote host.
# Usage: ./scripts/deploy/deploy-to-gcp.sh <host> [--user <user>]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOST="${1:?Usage: deploy-to-gcp.sh <host> [--user <user>]}"
USER="${3:-hypermesh}"
SSH="ssh -o StrictHostKeyChecking=accept-new"
SCP="scp -o StrictHostKeyChecking=accept-new"

RELEASE_DIR="$REPO_ROOT/target/release"

echo "=== Deploying HyperMesh to $HOST ==="

# 1. Ensure binaries exist
for bin in gateway trustchain_ca node catalog-server; do
    if [ ! -f "$RELEASE_DIR/$bin" ]; then
        echo "ERROR: $RELEASE_DIR/$bin not found. Run build-release.sh first."
        exit 1
    fi
done

# 2. Create remote directories
echo "Creating remote directories..."
$SSH "root@$HOST" bash -s <<'REMOTE'
set -euo pipefail
id -u hypermesh &>/dev/null || useradd -r -s /usr/sbin/nologin -d /var/lib/hypermesh hypermesh
mkdir -p /var/lib/hypermesh/{blockmatrix,trustchain,gateway,catalog}
mkdir -p /var/log/hypermesh
mkdir -p /etc/hypermesh/certs
chown -R hypermesh:hypermesh /var/lib/hypermesh /var/log/hypermesh /etc/hypermesh
REMOTE

# 3. Upload binaries
echo "Uploading binaries..."
$SCP "$RELEASE_DIR/gateway" "root@$HOST:/usr/local/bin/gateway"
$SCP "$RELEASE_DIR/trustchain_ca" "root@$HOST:/usr/local/bin/trustchain_ca"
$SCP "$RELEASE_DIR/node" "root@$HOST:/usr/local/bin/node"
$SCP "$RELEASE_DIR/catalog-server" "root@$HOST:/usr/local/bin/catalog-server"

# 4. Upload configs
echo "Uploading configs..."
$SCP "$REPO_ROOT/gateway/config/gateway.toml" "root@$HOST:/etc/hypermesh/gateway.toml"
$SCP "$REPO_ROOT/trustchain/config/production.toml" "root@$HOST:/etc/hypermesh/trustchain.toml"

# 5. Upload systemd units
echo "Uploading systemd units..."
$SCP "$REPO_ROOT/systemd/gateway.service" "root@$HOST:/etc/systemd/system/gateway.service"
$SCP "$REPO_ROOT/systemd/trustchain.service" "root@$HOST:/etc/systemd/system/trustchain.service"
$SCP "$REPO_ROOT/systemd/blockmatrix.service" "root@$HOST:/etc/systemd/system/blockmatrix.service"
$SCP "$REPO_ROOT/systemd/catalog.service" "root@$HOST:/etc/systemd/system/catalog.service"

# 6. Set permissions and reload
echo "Setting permissions and reloading systemd..."
$SSH "root@$HOST" bash -s <<'REMOTE'
set -euo pipefail
chmod 755 /usr/local/bin/{gateway,trustchain_ca,node,catalog-server}
systemctl daemon-reload
echo "Systemd units installed. Enable with:"
echo "  systemctl enable --now trustchain blockmatrix catalog gateway"
REMOTE

echo ""
echo "=== Deployment complete ==="
echo "Next steps:"
echo "  1. Set up TLS: ./scripts/deploy/setup-tls.sh $HOST"
echo "  2. Enable services: ssh root@$HOST 'systemctl enable --now trustchain blockmatrix catalog gateway'"
echo "  3. Verify: curl -k --http3 https://$HOST:8443/health"
