#!/bin/bash
# compile_ebpf.sh - Compile all eBPF programs for HyperMesh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}HyperMesh eBPF Program Compiler${NC}"
echo "================================"

# Check for required tools
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}Error: $1 is not installed${NC}"
        echo "Please install required tools first (see README.md)"
        exit 1
    fi
}

echo "Checking prerequisites..."
check_tool clang
check_tool llc

# Get kernel version for compatibility checks
KERNEL_VERSION=$(uname -r)
echo -e "Kernel version: ${YELLOW}$KERNEL_VERSION${NC}"

# Check for kernel headers
if [ ! -d "/lib/modules/$(uname -r)/build" ]; then
    echo -e "${YELLOW}Warning: Kernel headers not found${NC}"
    echo "You may need to install linux-headers-$(uname -r)"
fi

# Compile each C program
echo ""
echo "Compiling eBPF programs..."
echo "-------------------------"

compile_program() {
    local src="$1"
    local base="${src%.c}"
    local obj="${base}.o"

    echo -n "Compiling $src -> $obj ... "

    if clang -O2 -target bpf \
             -D__TARGET_ARCH_x86 \
             -I/usr/include/x86_64-linux-gnu \
             -I/usr/include \
             -c "$src" -o "$obj" 2>/dev/null; then
        echo -e "${GREEN}OK${NC}"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        echo "Trying with verbose output:"
        clang -O2 -target bpf \
              -D__TARGET_ARCH_x86 \
              -I/usr/include/x86_64-linux-gnu \
              -I/usr/include \
              -c "$src" -o "$obj"
        return 1
    fi
}

# Compile all .c files
SUCCESS_COUNT=0
FAIL_COUNT=0

for prog in *.c; do
    if [ -f "$prog" ]; then
        if compile_program "$prog"; then
            ((SUCCESS_COUNT++))
        else
            ((FAIL_COUNT++))
        fi
    fi
done

# Summary
echo ""
echo "Compilation Summary"
echo "==================="
echo -e "Successful: ${GREEN}$SUCCESS_COUNT${NC}"
echo -e "Failed: ${RED}$FAIL_COUNT${NC}"

if [ $FAIL_COUNT -eq 0 ]; then
    echo ""
    echo -e "${GREEN}All programs compiled successfully!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Ensure you have CAP_BPF capability or run as root"
    echo "2. Run: cargo test --test test_ebpf_kernel_integration"
    echo "3. Check dmesg for any kernel messages"
else
    echo ""
    echo -e "${RED}Some programs failed to compile${NC}"
    echo "Check the error messages above and ensure:"
    echo "- Kernel headers are installed"
    echo "- BPF headers are available"
    exit 1
fi

# List compiled programs
echo ""
echo "Compiled eBPF programs:"
ls -lh *.o 2>/dev/null || echo "No .o files found"