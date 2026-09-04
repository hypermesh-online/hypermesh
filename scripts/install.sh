#!/usr/bin/env bash
# Install HyperMesh node from GitHub Releases.
# Usage: curl -sSf https://raw.githubusercontent.com/hypermesh-online/hypermesh/main/scripts/install.sh | bash
#
# Options:
#   --version <tag>   Install specific version (default: latest)
#   --prefix <dir>    Install prefix (default: auto-detected)
#   --no-systemd      Skip systemd unit installation (Linux only)
#   --no-verify       Skip SHA-256 verification against release-manifest.json
#   --target <triple> Force a specific Rust target triple
#
# After install, use `hypermesh update` (Phase J release_feed_subscriber) to
# upgrade — install.sh is bootstrap-only. Subsequent updates flow through the
# foundation-signed release feed.
set -euo pipefail

VERSION=""
PREFIX=""
INSTALL_SYSTEMD=true
VERIFY=true
FORCE_TARGET=""
GITHUB_ORG="hypermesh-online"
GITHUB_REPO="hypermesh"
BINARIES="hypermesh"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)    VERSION="$2"; shift 2 ;;
        --prefix)     PREFIX="$2"; shift 2 ;;
        --no-systemd) INSTALL_SYSTEMD=false; shift ;;
        --no-verify)  VERIFY=false; shift ;;
        --target)     FORCE_TARGET="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: install.sh [--version <tag>] [--prefix <dir>] [--no-systemd] [--no-verify] [--target <triple>]"
            exit 0 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ---------------------------------------------------------------------------
# Detect target triple (Rust convention)
# ---------------------------------------------------------------------------
detect_target() {
    local arch os triple
    arch="$(uname -m)"
    os="$(uname -s)"

    case "$arch" in
        x86_64|amd64)         arch="x86_64" ;;
        aarch64|arm64)        arch="aarch64" ;;
        *)
            echo "ERROR: Unsupported architecture: $arch" >&2
            echo "Supported: x86_64, aarch64" >&2
            exit 1
            ;;
    esac

    case "$os" in
        Linux)
            # Default to musl for portable static binary
            triple="${arch}-unknown-linux-musl"
            ;;
        Darwin)
            triple="${arch}-apple-darwin"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "ERROR: Windows install via bash is not supported." >&2
            echo "Use PowerShell instead:" >&2
            echo "  iwr -useb https://raw.githubusercontent.com/$GITHUB_ORG/$GITHUB_REPO/main/scripts/install.ps1 | iex" >&2
            echo "or download hypermesh-<version>-x86_64-pc-windows-msvc.zip from:" >&2
            echo "  https://github.com/$GITHUB_ORG/$GITHUB_REPO/releases" >&2
            echo "and extract hypermesh.exe to a directory on your PATH." >&2
            exit 1
            ;;
        *)
            echo "ERROR: Unsupported OS: $os" >&2
            exit 1
            ;;
    esac
    echo "$triple"
}

if [ -n "$FORCE_TARGET" ]; then
    TARGET="$FORCE_TARGET"
else
    TARGET="$(detect_target)"
fi

# Pick a sensible default prefix per platform
if [ -z "$PREFIX" ]; then
    case "$TARGET" in
        aarch64-apple-darwin)
            # Apple Silicon Homebrew default
            if [ -d "/opt/homebrew" ]; then
                PREFIX="/opt/homebrew"
            else
                PREFIX="/usr/local"
            fi
            ;;
        *)
            PREFIX="/usr/local"
            ;;
    esac
fi

# ---------------------------------------------------------------------------
# Determine version
# ---------------------------------------------------------------------------
if [ -z "$VERSION" ]; then
    echo "Fetching latest release..."
    VERSION=$(curl -sSfL "https://api.github.com/repos/$GITHUB_ORG/$GITHUB_REPO/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)
    if [ -z "$VERSION" ]; then
        echo "ERROR: Could not determine latest version. Use --version to specify." >&2
        exit 1
    fi
fi

echo "Installing HyperMesh $VERSION for $TARGET..."

ARCHIVE="hypermesh-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/$GITHUB_ORG/$GITHUB_REPO/releases/download/$VERSION/$ARCHIVE"
MANIFEST_URL="https://github.com/$GITHUB_ORG/$GITHUB_REPO/releases/download/$VERSION/release-manifest.json"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# ---------------------------------------------------------------------------
# Download archive
# ---------------------------------------------------------------------------
echo "Downloading $DOWNLOAD_URL..."
if ! curl -sSfL "$DOWNLOAD_URL" -o "$TMPDIR/$ARCHIVE"; then
    echo "ERROR: Failed to download $ARCHIVE." >&2
    echo "Check that release $VERSION includes a build for $TARGET at:" >&2
    echo "  https://github.com/$GITHUB_ORG/$GITHUB_REPO/releases/tag/$VERSION" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# SHA-256 verification against release-manifest.json
# ---------------------------------------------------------------------------
if $VERIFY; then
    echo "Verifying SHA-256 against release-manifest.json..."
    if curl -sSfL "$MANIFEST_URL" -o "$TMPDIR/release-manifest.json" 2>/dev/null; then
        # Compute archive hash
        if command -v sha256sum >/dev/null 2>&1; then
            ARCHIVE_HASH=$(sha256sum "$TMPDIR/$ARCHIVE" | awk '{print $1}')
        else
            ARCHIVE_HASH=$(shasum -a 256 "$TMPDIR/$ARCHIVE" | awk '{print $1}')
        fi

        # Extract binary, hash it (manifest records the binary hash, not archive)
        tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"
        STAGE_DIR="$TMPDIR/hypermesh-${VERSION}-${TARGET}"
        if [ ! -d "$STAGE_DIR" ]; then
            STAGE_DIR="$TMPDIR"
        fi
        if command -v sha256sum >/dev/null 2>&1; then
            BIN_HASH=$(sha256sum "$STAGE_DIR/hypermesh" | awk '{print $1}')
        else
            BIN_HASH=$(shasum -a 256 "$STAGE_DIR/hypermesh" | awk '{print $1}')
        fi

        # Pull expected hash for this target. Prefer python3, fall back to grep+sed.
        EXPECTED=""
        if command -v python3 >/dev/null 2>&1; then
            EXPECTED=$(python3 -c "import json,sys; m=json.load(open('$TMPDIR/release-manifest.json')); print(m.get('binary_hashes',{}).get('$TARGET',''))")
        else
            EXPECTED=$(grep -oE "\"$TARGET\"[[:space:]]*:[[:space:]]*\"[0-9a-f]{64}\"" "$TMPDIR/release-manifest.json" | grep -oE '[0-9a-f]{64}' | head -1)
        fi

        if [ -z "$EXPECTED" ]; then
            echo "WARNING: No hash entry for $TARGET in release-manifest.json — skipping verification." >&2
        elif [ "$EXPECTED" != "$BIN_HASH" ]; then
            echo "ERROR: SHA-256 mismatch for hypermesh binary ($TARGET):" >&2
            echo "  expected: $EXPECTED" >&2
            echo "  actual:   $BIN_HASH" >&2
            exit 1
        else
            echo "  OK ($BIN_HASH)"
        fi

        # Foundation signature check is not done by install.sh. The release_feed
        # subscriber inside the running daemon validates FALCON-1024 signatures
        # against the configured foundation pubkey. install.sh only verifies
        # archive integrity (SHA-256 from the unsigned manifest) so a tampered
        # archive on the GitHub release page is detected at install time.
        if grep -q '"signature": ""' "$TMPDIR/release-manifest.json" 2>/dev/null; then
            echo "  NOTE: release-manifest.json is not yet foundation-signed."
            echo "        Daemon FALCON-1024 verification will run on first feed poll."
        fi
    else
        echo "WARNING: release-manifest.json not found in release — skipping verification." >&2
        # Extract anyway (legacy releases may not have a manifest)
        tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"
        STAGE_DIR="$TMPDIR/hypermesh-${VERSION}-${TARGET}"
        if [ ! -d "$STAGE_DIR" ]; then
            STAGE_DIR="$TMPDIR"
        fi
    fi
else
    tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"
    STAGE_DIR="$TMPDIR/hypermesh-${VERSION}-${TARGET}"
    if [ ! -d "$STAGE_DIR" ]; then
        STAGE_DIR="$TMPDIR"
    fi
fi

# ---------------------------------------------------------------------------
# Install binaries
# ---------------------------------------------------------------------------
echo "Installing binaries to $PREFIX/bin/..."
NEED_SUDO=""
if [ ! -w "$PREFIX/bin" ] 2>/dev/null && [ "$(id -u)" -ne 0 ]; then
    if command -v sudo >/dev/null 2>&1; then
        NEED_SUDO="sudo"
        echo "  ($PREFIX/bin not writable — using sudo)"
    fi
fi

$NEED_SUDO install -d "$PREFIX/bin"
for bin in $BINARIES; do
    SRC=""
    for candidate in "$STAGE_DIR/$bin" "$STAGE_DIR/bin/$bin" "$TMPDIR/$bin"; do
        if [ -f "$candidate" ]; then
            SRC="$candidate"
            break
        fi
    done
    if [ -n "$SRC" ]; then
        $NEED_SUDO install -m 755 "$SRC" "$PREFIX/bin/$bin"
        echo "  Installed $bin"
    else
        echo "  WARNING: $bin not found in archive"
    fi
done

# ---------------------------------------------------------------------------
# Linux-only: system user, dirs, systemd
# ---------------------------------------------------------------------------
case "$TARGET" in
    *-linux-*)
        if [ "$(id -u)" -eq 0 ]; then
            echo "Setting up hypermesh user and directories..."
            id -u hypermesh >/dev/null 2>&1 || \
                useradd -r -s /usr/sbin/nologin -d /var/lib/hypermesh hypermesh
            install -d -o hypermesh -g hypermesh \
                /var/lib/hypermesh/{blockmatrix,trustchain,gateway,catalog,ngauge,caesar}
            install -d -o hypermesh -g hypermesh /var/log/hypermesh
            install -d -o hypermesh -g hypermesh /etc/hypermesh/certs
        else
            echo "Not running as root — skipping user/directory creation."
            echo "  Manually create: hypermesh user, /var/lib/hypermesh, /var/log/hypermesh, /etc/hypermesh"
        fi

        if $INSTALL_SYSTEMD && [ "$(id -u)" -eq 0 ] && command -v systemctl >/dev/null 2>&1; then
            echo "Installing systemd units..."
            SYSTEMD_DIR="/etc/systemd/system"
            for unit in gateway.service trustchain.service blockmatrix.service catalog.service ngauge.service caesar.service; do
                if [ -f "$STAGE_DIR/systemd/$unit" ]; then
                    install -m 644 "$STAGE_DIR/systemd/$unit" "$SYSTEMD_DIR/$unit"
                    echo "  Installed $unit"
                elif [ -f "$STAGE_DIR/$unit" ]; then
                    install -m 644 "$STAGE_DIR/$unit" "$SYSTEMD_DIR/$unit"
                    echo "  Installed $unit"
                fi
            done
            systemctl daemon-reload
            echo
            echo "Start services with:"
            echo "  systemctl enable --now trustchain blockmatrix caesar catalog ngauge gateway"
        elif ! $INSTALL_SYSTEMD; then
            echo "Skipping systemd installation (--no-systemd)."
        fi
        ;;
    *-darwin)
        echo "macOS install complete. To run as a launchd service, create a"
        echo "  ~/Library/LaunchAgents/online.hypermesh.node.plist invoking:"
        echo "    $PREFIX/bin/hypermesh"
        ;;
esac

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
echo
echo "=== HyperMesh $VERSION installed for $TARGET ==="
echo "Binary:   $PREFIX/bin/hypermesh"
case "$TARGET" in
    *-linux-*)
        echo "Config:   /etc/hypermesh/"
        echo "Data:     /var/lib/hypermesh/"
        echo "Logs:     /var/log/hypermesh/"
        ;;
esac
echo
echo "Future updates: use \`hypermesh update\` (foundation release feed)."
