#!/usr/bin/env bash
# Set up TLS certificates via Let's Encrypt + Cloudflare DNS-01 challenge.
# Usage: ./scripts/deploy/setup-tls.sh [--domain <domain>] [--email <email>]
#
# Prerequisites:
#   - certbot and python3-certbot-dns-cloudflare installed on the target
#   - Cloudflare API token with DNS edit permissions in /etc/hypermesh/cloudflare.ini
set -euo pipefail

DOMAIN="${1:-trust.hypermesh.online}"
EMAIL="${2:-dev@hypermesh.online}"
CERT_DIR="/etc/hypermesh/certs"
CF_CREDENTIALS="/etc/hypermesh/cloudflare.ini"

echo "=== TLS Setup for $DOMAIN ==="

# 1. Check certbot
if ! command -v certbot &>/dev/null; then
    echo "Installing certbot and Cloudflare DNS plugin..."
    apt-get update -qq
    apt-get install -y -qq certbot python3-certbot-dns-cloudflare
fi

# 2. Check Cloudflare credentials
if [ ! -f "$CF_CREDENTIALS" ]; then
    echo "ERROR: Cloudflare credentials not found at $CF_CREDENTIALS"
    echo "Create it with:"
    echo "  echo 'dns_cloudflare_api_token = YOUR_TOKEN_HERE' > $CF_CREDENTIALS"
    echo "  chmod 600 $CF_CREDENTIALS"
    exit 1
fi

# 3. Obtain certificate
echo "Requesting certificate for $DOMAIN..."
certbot certonly \
    --dns-cloudflare \
    --dns-cloudflare-credentials "$CF_CREDENTIALS" \
    --dns-cloudflare-propagation-seconds 30 \
    -d "$DOMAIN" \
    --email "$EMAIL" \
    --agree-tos \
    --non-interactive \
    --key-type ecdsa \
    --elliptic-curve secp384r1

# 4. Copy to gateway cert directory
LIVE_DIR="/etc/letsencrypt/live/$DOMAIN"
echo "Copying certificates to $CERT_DIR..."
cp "$LIVE_DIR/fullchain.pem" "$CERT_DIR/server.crt"
cp "$LIVE_DIR/privkey.pem" "$CERT_DIR/server.key"
chown hypermesh:hypermesh "$CERT_DIR/server.crt" "$CERT_DIR/server.key"
chmod 640 "$CERT_DIR/server.crt" "$CERT_DIR/server.key"

# 5. Set up auto-renewal hook
RENEWAL_HOOK="/etc/letsencrypt/renewal-hooks/deploy/hypermesh-copy-certs.sh"
cat > "$RENEWAL_HOOK" <<'HOOK'
#!/bin/bash
cp /etc/letsencrypt/live/trust.hypermesh.online/fullchain.pem /etc/hypermesh/certs/server.crt
cp /etc/letsencrypt/live/trust.hypermesh.online/privkey.pem /etc/hypermesh/certs/server.key
chown hypermesh:hypermesh /etc/hypermesh/certs/server.crt /etc/hypermesh/certs/server.key
systemctl reload gateway 2>/dev/null || systemctl restart gateway
HOOK
chmod 755 "$RENEWAL_HOOK"

echo ""
echo "=== TLS Setup Complete ==="
echo "Certificate: $CERT_DIR/server.crt"
echo "Key:         $CERT_DIR/server.key"
echo "Auto-renewal: enabled (certbot timer)"
