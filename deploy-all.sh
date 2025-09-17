#!/bin/bash
#
# Quick Deploy Script - Sync all Web3 ecosystem repositories
#
# This script performs a complete sync of all components to their respective GitHub repositories
# with proper separation of concerns.

set -e

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}🚀 Web3 Ecosystem - Deploy All Repositories${NC}"
echo "=============================================="
echo
echo -e "${YELLOW}Components to deploy:${NC}"
echo "  • NGauge (engagement platform)"
echo "  • Caesar (economic layer)"
echo "  • Catalog (asset SDK)"
echo "  • HyperMesh (core platform)"
echo "  • STOQ (transport protocol)"
echo "  • TrustChain (certificate authority)"
echo
echo -e "${BLUE}GitHub Organization:${NC} https://github.com/hypermesh-online"
echo

# Check if sync script exists
if [[ ! -f "./sync-repos.sh" ]]; then
    echo "❌ Error: sync-repos.sh not found in current directory"
    echo "Please run this script from the Web3 ecosystem root directory"
    exit 1
fi

# Ask for confirmation unless --yes flag is provided
if [[ "$1" != "--yes" ]]; then
    echo -e "${YELLOW}This will sync all components to GitHub. Continue? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "Deployment cancelled."
        exit 0
    fi
fi

echo -e "${GREEN}Starting deployment...${NC}"
echo

# Execute sync script
./sync-repos.sh

echo
echo -e "${GREEN}✅ Deployment complete!${NC}"
echo
echo "📍 Repository links:"
echo "  • NGauge:     https://github.com/hypermesh-online/ngauge"
echo "  • Caesar:     https://github.com/hypermesh-online/caesar"
echo "  • Catalog:    https://github.com/hypermesh-online/catalog"
echo "  • HyperMesh:  https://github.com/hypermesh-online/hypermesh"
echo "  • STOQ:       https://github.com/hypermesh-online/stoq"
echo "  • TrustChain: https://github.com/hypermesh-online/trustchain"
echo
echo -e "${BLUE}🌐 Complete ecosystem: https://github.com/hypermesh-online${NC}"
echo
echo "Next steps:"
echo "  1. Review repositories on GitHub"
echo "  2. Set up CI/CD pipelines (GitHub Actions)"
echo "  3. Configure repository settings and permissions"
echo "  4. Begin staged production deployment"