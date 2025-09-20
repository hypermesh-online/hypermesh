#!/bin/bash
#
# UI Build Script for HyperMesh Dashboard
#
# Builds the frontend UI for production deployment
#

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}Building HyperMesh Dashboard UI...${NC}"

# Navigate to UI directory
cd ui/frontend

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}Installing dependencies...${NC}"
    npm install
fi

# Build for production
echo -e "${BLUE}Building production assets...${NC}"
NODE_ENV=production npm run build

if [ $? -eq 0 ]; then
    echo -e "${GREEN} UI build completed successfully!${NC}"
    echo -e "${GREEN}=æ Built assets are in: ui/frontend/dist/${NC}"

    # Show build size
    if command -v du &> /dev/null; then
        SIZE=$(du -sh dist | cut -f1)
        echo -e "${BLUE}=Ê Build size: ${SIZE}${NC}"
    fi

    # List main files
    echo -e "${BLUE}=Á Main files:${NC}"
    ls -lh dist/index.html 2>/dev/null || true
    ls -lh dist/assets/*.js 2>/dev/null | head -5 || true
else
    echo -e "${RED}L UI build failed${NC}"
    exit 1
fi

echo -e "${GREEN}<¨ UI is ready for deployment!${NC}"