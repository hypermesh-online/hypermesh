#!/bin/bash
#
# Local DNS Setup Script for HyperMesh
# Sets up /etc/hosts entries for local testing of the HyperMesh ecosystem
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
IPV6_ADDRESS="::1"  # IPv6 loopback for local testing
IPV4_ADDRESS="127.0.0.1"  # IPv4 fallback if needed

# Domains to configure
DOMAINS=(
    "hypermesh.online"
    "trust.hypermesh.online"
    "caesar.hypermesh.online"
    "catalog.hypermesh.online"
    "stoq.hypermesh.online"
    "ngauge.hypermesh.online"
)

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running with sudo
check_sudo() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run with sudo"
        exit 1
    fi
}

# Backup /etc/hosts
backup_hosts() {
    if [[ ! -f /etc/hosts.hypermesh.backup ]]; then
        cp /etc/hosts /etc/hosts.hypermesh.backup
        print_success "Created backup of /etc/hosts at /etc/hosts.hypermesh.backup"
    else
        print_status "Backup already exists at /etc/hosts.hypermesh.backup"
    fi
}

# Add entries to /etc/hosts
add_entries() {
    print_status "Adding HyperMesh domains to /etc/hosts..."

    # Add marker for our entries
    if ! grep -q "# BEGIN HYPERMESH LOCAL DNS" /etc/hosts; then
        echo "" >> /etc/hosts
        echo "# BEGIN HYPERMESH LOCAL DNS" >> /etc/hosts
    fi

    for domain in "${DOMAINS[@]}"; do
        # Remove any existing entries for this domain
        sed -i.tmp "/$domain/d" /etc/hosts

        # Add new entries (both IPv6 and IPv4 for compatibility)
        echo "$IPV6_ADDRESS    $domain" >> /etc/hosts
        echo "$IPV4_ADDRESS    $domain" >> /etc/hosts

        print_success "Added $domain → $IPV6_ADDRESS and $IPV4_ADDRESS"
    done

    # Add end marker
    if ! grep -q "# END HYPERMESH LOCAL DNS" /etc/hosts; then
        echo "# END HYPERMESH LOCAL DNS" >> /etc/hosts
    fi
}

# Remove entries from /etc/hosts
remove_entries() {
    print_status "Removing HyperMesh domains from /etc/hosts..."

    # Remove all entries between our markers
    sed -i.tmp '/# BEGIN HYPERMESH LOCAL DNS/,/# END HYPERMESH LOCAL DNS/d' /etc/hosts

    # Also remove any stray entries
    for domain in "${DOMAINS[@]}"; do
        sed -i.tmp "/$domain/d" /etc/hosts
    done

    print_success "Removed HyperMesh domains from /etc/hosts"
}

# Test DNS resolution
test_resolution() {
    print_status "Testing DNS resolution..."
    echo ""

    for domain in "${DOMAINS[@]}"; do
        # Test IPv6 resolution
        if ping6 -c 1 -W 1 "$domain" &>/dev/null; then
            echo -e "  ${GREEN}✓${NC} $domain (IPv6)"
        else
            echo -e "  ${YELLOW}⚠${NC} $domain (IPv6 not responding)"
        fi

        # Test IPv4 resolution
        if ping -c 1 -W 1 "$domain" &>/dev/null; then
            echo -e "  ${GREEN}✓${NC} $domain (IPv4)"
        else
            echo -e "  ${YELLOW}⚠${NC} $domain (IPv4 not responding)"
        fi
    done

    echo ""
}

# Show usage
usage() {
    echo "Usage: sudo $0 {setup|remove|test|restore}"
    echo ""
    echo "Commands:"
    echo "  setup    - Add HyperMesh domains to /etc/hosts"
    echo "  remove   - Remove HyperMesh domains from /etc/hosts"
    echo "  test     - Test DNS resolution for HyperMesh domains"
    echo "  restore  - Restore /etc/hosts from backup"
    exit 1
}

# Main execution
case "${1:-}" in
    setup)
        check_sudo
        backup_hosts
        add_entries
        test_resolution
        print_success "Local DNS setup complete!"
        echo ""
        echo "You can now access the HyperMesh ecosystem at:"
        for domain in "${DOMAINS[@]}"; do
            echo "  https://$domain:8443"
        done
        ;;

    remove)
        check_sudo
        remove_entries
        print_success "HyperMesh domains removed from /etc/hosts"
        ;;

    test)
        test_resolution
        ;;

    restore)
        check_sudo
        if [[ -f /etc/hosts.hypermesh.backup ]]; then
            cp /etc/hosts.hypermesh.backup /etc/hosts
            print_success "Restored /etc/hosts from backup"
        else
            print_error "No backup found at /etc/hosts.hypermesh.backup"
            exit 1
        fi
        ;;

    *)
        usage
        ;;
esac