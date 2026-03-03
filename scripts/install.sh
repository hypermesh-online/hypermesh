#!/usr/bin/env bash
# Install HyperMesh node from GitHub Releases.
# Usage: curl -sSf https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.sh | bash
#
# Options:
#   --version <tag>   Install specific version (default: latest)
#   --prefix <dir>    Install prefix (default: /usr/local)
#   --no-systemd      Skip systemd unit installation
set -euo pipefail

VERSION=""
PREFIX="/usr/local"
INSTALL_SYSTEMD=true
GITHUB_ORG="hypermesh-online"
GITHUB_REPO="core"
BINARIES="gateway trustchain_ca hypermesh catalog-server"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)  VERSION="$2"; shift 2 ;;
        --prefix)   PREFIX="$2"; shift 2 ;;
        --no-systemd) INSTALL_SYSTEMD=false; shift ;;
        -h|--help)
            echo "Usage: install.sh [--version <tag>] [--prefix <dir>] [--no-systemd]"
            exit 0 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    *)
        echo "ERROR: Unsupported architecture: $ARCH (only x86_64 supported)"
        exit 1
        ;;
esac

OS="$(uname -s)"
case "$OS" in
    Linux) OS="linux" ;;
    *)
        echo "ERROR: Unsupported OS: $OS (only Linux supported)"
        exit 1
        ;;
esac

# Determine version
if [ -z "$VERSION" ]; then
    echo "Fetching latest release..."
    VERSION=$(curl -sSf "https://api.github.com/repos/$GITHUB_ORG/$GITHUB_REPO/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)
    if [ -z "$VERSION" ]; then
        echo "ERROR: Could not determine latest version. Use --version to specify."
        exit 1
    fi
fi

echo "Installing HyperMesh $VERSION for $ARCH-$OS..."

TARBALL="hypermesh-${VERSION}-${ARCH}-${OS}.tar.gz"
DOWNLOAD_URL="https://github.com/$GITHUB_ORG/$GITHUB_REPO/releases/download/$VERSION/$TARBALL"

# Download and extract
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $DOWNLOAD_URL..."
curl -sSfL "$DOWNLOAD_URL" -o "$TMPDIR/$TARBALL"
tar xzf "$TMPDIR/$TARBALL" -C "$TMPDIR"

# Install binaries
echo "Installing binaries to $PREFIX/bin/..."
install -d "$PREFIX/bin"
for bin in $BINARIES; do
    if [ -f "$TMPDIR/$bin" ]; then
        install -m 755 "$TMPDIR/$bin" "$PREFIX/bin/$bin"
        echo "  Installed $bin"
    elif [ -f "$TMPDIR/bin/$bin" ]; then
        install -m 755 "$TMPDIR/bin/$bin" "$PREFIX/bin/$bin"
        echo "  Installed $bin"
    else
        echo "  WARNING: $bin not found in release"
    fi
done

# Create system user and directories
if [ "$(id -u)" -eq 0 ]; then
    echo "Setting up hypermesh user and directories..."
    id -u hypermesh &>/dev/null 2>&1 || useradd -r -s /usr/sbin/nologin -d /var/lib/hypermesh hypermesh
    install -d -o hypermesh -g hypermesh /var/lib/hypermesh/{blockmatrix,trustchain,gateway,catalog}
    install -d -o hypermesh -g hypermesh /var/log/hypermesh
    install -d -o hypermesh -g hypermesh /etc/hypermesh/certs
else
    echo "Not running as root — skipping user/directory creation."
    echo "  Run as root or manually create: hypermesh user, /var/lib/hypermesh, /var/log/hypermesh, /etc/hypermesh"
fi

# Install systemd units
if $INSTALL_SYSTEMD && [ "$(id -u)" -eq 0 ] && command -v systemctl &>/dev/null; then
    echo "Installing systemd units..."
    SYSTEMD_DIR="/etc/systemd/system"
    for unit in gateway.service trustchain.service blockmatrix.service catalog.service; do
        if [ -f "$TMPDIR/systemd/$unit" ]; then
            install -m 644 "$TMPDIR/systemd/$unit" "$SYSTEMD_DIR/$unit"
            echo "  Installed $unit"
        elif [ -f "$TMPDIR/$unit" ]; then
            install -m 644 "$TMPDIR/$unit" "$SYSTEMD_DIR/$unit"
            echo "  Installed $unit"
        fi
    done
    systemctl daemon-reload
    echo ""
    echo "Start services with:"
    echo "  systemctl enable --now trustchain blockmatrix catalog gateway"
elif ! $INSTALL_SYSTEMD; then
    echo "Skipping systemd installation (--no-systemd)."
else
    echo "Skipping systemd installation (not root or systemctl not found)."
fi

echo ""
echo "=== HyperMesh $VERSION installed ==="
echo "Binaries: $PREFIX/bin/{$(echo $BINARIES | tr ' ' ',')}"
echo "Config:   /etc/hypermesh/"
echo "Data:     /var/lib/hypermesh/"
echo "Logs:     /var/log/hypermesh/"
