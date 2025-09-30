#!/bin/bash
# Automated test scenarios for Catalog plugin lifecycle
# This script validates all phases of plugin loading, operation, and unloading

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
CATALOG_DIR="../catalog"
HYPERMESH_DIR="../hypermesh"
TEST_DIR="./test_output"
LOG_FILE="$TEST_DIR/plugin_lifecycle_$(date +%Y%m%d_%H%M%S).log"

# Create test directory
mkdir -p "$TEST_DIR"

# Logging function
log() {
    echo -e "$1" | tee -a "$LOG_FILE"
}

# Test result tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Test execution function
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_result="${3:-0}"

    log "${BLUE}[TEST] Running: $test_name${NC}"

    if eval "$test_command" >> "$LOG_FILE" 2>&1; then
        if [ "$expected_result" -eq 0 ]; then
            log "${GREEN}[PASS] $test_name${NC}"
            ((TESTS_PASSED++))
            return 0
        else
            log "${RED}[FAIL] $test_name - Expected failure but succeeded${NC}"
            ((TESTS_FAILED++))
            return 1
        fi
    else
        local exit_code=$?
        if [ "$expected_result" -ne 0 ]; then
            log "${GREEN}[PASS] $test_name - Failed as expected${NC}"
            ((TESTS_PASSED++))
            return 0
        else
            log "${RED}[FAIL] $test_name - Exit code: $exit_code${NC}"
            ((TESTS_FAILED++))
            return 1
        fi
    fi
}

# Header
log "${BLUE}================================================${NC}"
log "${BLUE}    Catalog Plugin Lifecycle Test Suite${NC}"
log "${BLUE}================================================${NC}"
log "Test started at: $(date)"
log "Log file: $LOG_FILE"
log ""

# Phase 1: Build Verification
log "${YELLOW}=== Phase 1: Build Verification ===${NC}"

run_test "Catalog library build" \
    "cd $CATALOG_DIR && cargo build --lib --release"

run_test "HyperMesh build" \
    "cd $HYPERMESH_DIR && cargo build --release"

run_test "Verify plugin binary exists" \
    "test -f $CATALOG_DIR/target/release/libcatalog.so || test -f $CATALOG_DIR/target/release/libcatalog.dylib || test -f $CATALOG_DIR/target/release/catalog.dll"

run_test "Verify manifest exists" \
    "test -f $CATALOG_DIR/manifest.json || test -f $CATALOG_DIR/extension.json"

# Phase 2: Plugin Discovery
log ""
log "${YELLOW}=== Phase 2: Plugin Discovery ===${NC}"

run_test "Plugin discovery in debug directory" \
    "cd $HYPERMESH_DIR && cargo test test_plugin_discovery_multiple_paths -- --nocapture"

run_test "Manifest validation" \
    "cd $HYPERMESH_DIR && cargo test test_manifest_validation -- --nocapture"

# Phase 3: Plugin Loading
log ""
log "${YELLOW}=== Phase 3: Plugin Loading ===${NC}"

run_test "Basic plugin loading" \
    "cd $HYPERMESH_DIR && cargo test test_catalog_extension_loading -- --nocapture"

run_test "Signature verification (if enabled)" \
    "cd $HYPERMESH_DIR && cargo test test_signature_verification -- --nocapture"

run_test "Capability-based security" \
    "cd $HYPERMESH_DIR && cargo test test_capability_based_security -- --nocapture"

run_test "Configuration scenarios" \
    "cd $HYPERMESH_DIR && cargo test test_configuration_scenarios -- --nocapture"

# Phase 4: Extension Integration
log ""
log "${YELLOW}=== Phase 4: Extension Integration ===${NC}"

run_test "Extension trait implementation" \
    "cd $HYPERMESH_DIR && cargo test test_extension_trait_implementation -- --nocapture"

run_test "Asset type registration" \
    "cd $HYPERMESH_DIR && cargo test test_asset_registration -- --nocapture"

run_test "Asset handlers integration" \
    "cd $HYPERMESH_DIR && cargo test test_asset_handlers -- --nocapture"

run_test "API endpoints accessibility" \
    "cd $HYPERMESH_DIR && cargo test test_api_endpoints -- --nocapture"

run_test "Consensus integration" \
    "cd $HYPERMESH_DIR && cargo test test_consensus_integration -- --nocapture"

# Phase 5: Runtime Operations
log ""
log "${YELLOW}=== Phase 5: Runtime Operations ===${NC}"

run_test "Asset library operations" \
    "cd $HYPERMESH_DIR && cargo test test_library_operations -- --nocapture"

run_test "P2P distribution" \
    "cd $HYPERMESH_DIR && cargo test test_p2p_distribution -- --nocapture"

run_test "TrustChain verification" \
    "cd $HYPERMESH_DIR && cargo test test_trustchain_verification -- --nocapture"

run_test "Cross-node synchronization" \
    "cd $HYPERMESH_DIR && cargo test test_cross_node_sync -- --nocapture"

run_test "Resource isolation" \
    "cd $HYPERMESH_DIR && cargo test test_resource_isolation -- --nocapture"

# Phase 6: Full System Tests
log ""
log "${YELLOW}=== Phase 6: Full System Tests ===${NC}"

run_test "Full system initialization" \
    "cd $CATALOG_DIR && cargo test test_full_system_initialization -- --nocapture"

run_test "Asset library management" \
    "cd $CATALOG_DIR && cargo test test_asset_library_management -- --nocapture"

run_test "Package lifecycle" \
    "cd $CATALOG_DIR && cargo test test_package_lifecycle -- --nocapture"

run_test "VM execution" \
    "cd $CATALOG_DIR && cargo test test_vm_execution -- --nocapture"

run_test "Consensus validation" \
    "cd $CATALOG_DIR && cargo test test_consensus_validation -- --nocapture"

# Phase 7: Performance Tests
log ""
log "${YELLOW}=== Phase 7: Performance Tests ===${NC}"

run_test "Performance under load" \
    "cd $CATALOG_DIR && cargo test test_performance_under_load -- --nocapture"

run_test "Concurrent operations" \
    "cd $HYPERMESH_DIR && cargo test test_concurrent_operations -- --nocapture"

# Phase 8: Resilience Tests
log ""
log "${YELLOW}=== Phase 8: Resilience Tests ===${NC}"

run_test "Cleanup on unload" \
    "cd $HYPERMESH_DIR && cargo test test_cleanup_on_unload -- --nocapture"

run_test "State persistence" \
    "cd $HYPERMESH_DIR && cargo test test_state_persistence -- --nocapture"

run_test "Hot reload functionality" \
    "cd $HYPERMESH_DIR && cargo test test_hot_reload -- --nocapture"

run_test "Recovery and resilience" \
    "cd $CATALOG_DIR && cargo test test_recovery_resilience -- --nocapture"

# Phase 9: Error Handling
log ""
log "${YELLOW}=== Phase 9: Error Handling ===${NC}"

run_test "Corrupted plugin handling" \
    "cd $HYPERMESH_DIR && cargo test test_corrupted_plugin -- --nocapture"

run_test "Missing dependencies" \
    "cd $HYPERMESH_DIR && cargo test test_missing_dependencies -- --nocapture"

run_test "Network partition handling" \
    "cd $HYPERMESH_DIR && cargo test test_network_partition -- --nocapture"

run_test "Crash recovery" \
    "cd $HYPERMESH_DIR && cargo test test_crash_recovery -- --nocapture"

# Phase 10: Multi-Node Tests (if environment supports it)
log ""
log "${YELLOW}=== Phase 10: Multi-Node Tests ===${NC}"

# Check if multi-node environment is available
if command -v docker &> /dev/null; then
    run_test "P2P distribution across nodes" \
        "cd $CATALOG_DIR && cargo test test_p2p_distribution -- --nocapture"

    run_test "Multi-node coordination" \
        "cd $CATALOG_DIR && cargo test test_multi_node_coordination -- --nocapture"
else
    log "${YELLOW}[SKIP] Multi-node tests - Docker not available${NC}"
    ((TESTS_SKIPPED+=2))
fi

# Memory leak test (optional, takes longer)
if [ "${RUN_MEMORY_TESTS:-0}" -eq 1 ]; then
    log ""
    log "${YELLOW}=== Memory Leak Tests ===${NC}"

    run_test "Memory leak detection" \
        "cd $HYPERMESH_DIR && cargo test test_memory_leaks -- --ignored --nocapture"
else
    log "${YELLOW}[SKIP] Memory leak tests - Set RUN_MEMORY_TESTS=1 to enable${NC}"
    ((TESTS_SKIPPED++))
fi

# Summary
log ""
log "${BLUE}================================================${NC}"
log "${BLUE}                Test Summary${NC}"
log "${BLUE}================================================${NC}"
log "${GREEN}Passed: $TESTS_PASSED${NC}"
log "${RED}Failed: $TESTS_FAILED${NC}"
log "${YELLOW}Skipped: $TESTS_SKIPPED${NC}"
log "Total: $((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))"
log ""

# Generate detailed report
REPORT_FILE="$TEST_DIR/test_report_$(date +%Y%m%d_%H%M%S).json"
cat > "$REPORT_FILE" <<EOF
{
    "timestamp": "$(date -Iseconds)",
    "summary": {
        "passed": $TESTS_PASSED,
        "failed": $TESTS_FAILED,
        "skipped": $TESTS_SKIPPED,
        "total": $((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))
    },
    "environment": {
        "rust_version": "$(rustc --version)",
        "cargo_version": "$(cargo --version)",
        "os": "$(uname -s)",
        "arch": "$(uname -m)"
    },
    "log_file": "$LOG_FILE"
}
EOF

log "Detailed report saved to: $REPORT_FILE"

# Exit with appropriate code
if [ $TESTS_FAILED -gt 0 ]; then
    log "${RED}TEST SUITE FAILED${NC}"
    exit 1
else
    log "${GREEN}TEST SUITE PASSED${NC}"
    exit 0
fi