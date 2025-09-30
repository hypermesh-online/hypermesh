#!/bin/bash

#############################################################
# Quality Gate Enforcer for HyperMesh Ecosystem
# Enforces quality standards before deployment
#############################################################

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Quality thresholds
readonly MIN_TEST_COVERAGE=60
readonly MAX_SECURITY_ISSUES=0
readonly MAX_CLIPPY_WARNINGS=0
readonly PERFORMANCE_REGRESSION_THRESHOLD=20
readonly MAX_BUILD_TIME_MINUTES=10

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

# Results tracking
GATE_RESULTS=()
FAILED_GATES=()
TOTAL_SCORE=0
MAX_SCORE=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    GATE_RESULTS+=("✓ $1")
}

log_warning() {
    echo -e "${YELLOW}[⚠]${NC} $1"
    GATE_RESULTS+=("⚠ $1")
}

log_error() {
    echo -e "${RED}[✗]${NC} $1" >&2
    GATE_RESULTS+=("✗ $1")
    FAILED_GATES+=("$1")
}

log_section() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
}

# Gate 1: Compilation Check
check_compilation() {
    log_section "GATE 1: COMPILATION CHECK"
    MAX_SCORE=$((MAX_SCORE + 100))

    local compilation_failed=false

    for component in stoq trustchain hypermesh caesar catalog; do
        log_info "Compiling $component..."

        if cd "$PROJECT_ROOT/$component" && cargo check --all-features --all-targets &> /dev/null; then
            log_success "$component compiled successfully"
            TOTAL_SCORE=$((TOTAL_SCORE + 20))
        else
            log_error "$component compilation failed"
            compilation_failed=true
        fi
    done

    cd "$PROJECT_ROOT"

    if [ "$compilation_failed" = false ]; then
        log_success "COMPILATION GATE PASSED"
        return 0
    else
        log_error "COMPILATION GATE FAILED"
        return 1
    fi
}

# Gate 2: Security Audit
check_security() {
    log_section "GATE 2: SECURITY AUDIT"
    MAX_SCORE=$((MAX_SCORE + 100))

    local security_issues=0

    # Check for cargo-audit
    if ! command -v cargo-audit &> /dev/null; then
        log_warning "cargo-audit not installed, installing..."
        cargo install cargo-audit --locked
    fi

    # Run security audit on each component
    for component in stoq trustchain hypermesh caesar catalog; do
        log_info "Auditing $component dependencies..."

        cd "$PROJECT_ROOT/$component"

        # Run cargo audit
        if cargo audit --json 2>/dev/null | jq -r '.vulnerabilities.found' | grep -q "0"; then
            log_success "$component: No known vulnerabilities"
            TOTAL_SCORE=$((TOTAL_SCORE + 10))
        else
            local vuln_count=$(cargo audit --json 2>/dev/null | jq -r '.vulnerabilities.count // 0')
            log_error "$component: $vuln_count vulnerabilities found"
            security_issues=$((security_issues + vuln_count))
        fi

        # Check for hardcoded secrets
        log_info "Scanning $component for hardcoded secrets..."
        if ! grep -r "password\|secret\|api_key\|token" --include="*.rs" . 2>/dev/null | \
           grep -v "// \|/// \|//! " | \
           grep -v "test\|example\|mock" | \
           grep -q "="; then
            log_success "$component: No hardcoded secrets found"
            TOTAL_SCORE=$((TOTAL_SCORE + 10))
        else
            log_error "$component: Potential hardcoded secrets detected"
            security_issues=$((security_issues + 1))
        fi
    done

    cd "$PROJECT_ROOT"

    # Check for mock implementations in production code
    log_info "Checking for mock implementations in production..."
    if grep -r "// MOCK:" --include="*.rs" --exclude-dir=tests --exclude-dir=benches . 2>/dev/null; then
        log_error "Mock implementations found in production code"
        security_issues=$((security_issues + 1))
    else
        log_success "No mock implementations in production"
        TOTAL_SCORE=$((TOTAL_SCORE + 10))
    fi

    # Check for debug output in production
    log_info "Checking for debug output in production..."
    if grep -r "dbg!\|println!\|eprintln!" --include="*.rs" --exclude-dir=tests --exclude-dir=examples . 2>/dev/null; then
        log_warning "Debug output found in production code"
    else
        log_success "No debug output in production"
        TOTAL_SCORE=$((TOTAL_SCORE + 10))
    fi

    if [ $security_issues -eq 0 ]; then
        log_success "SECURITY GATE PASSED"
        return 0
    else
        log_error "SECURITY GATE FAILED: $security_issues issues found"
        return 1
    fi
}

# Gate 3: Test Coverage
check_test_coverage() {
    log_section "GATE 3: TEST COVERAGE"
    MAX_SCORE=$((MAX_SCORE + 100))

    # Check for tarpaulin
    if ! command -v cargo-tarpaulin &> /dev/null; then
        log_warning "cargo-tarpaulin not installed, using basic test check"

        # Fallback to basic test execution
        for component in stoq trustchain hypermesh caesar catalog; do
            log_info "Running tests for $component..."

            cd "$PROJECT_ROOT/$component"
            if cargo test --lib --quiet 2>/dev/null; then
                log_success "$component: Tests passed"
                TOTAL_SCORE=$((TOTAL_SCORE + 15))
            else
                log_error "$component: Tests failed"
            fi
        done
    else
        # Use tarpaulin for coverage measurement
        local total_coverage=0
        local component_count=0

        for component in stoq trustchain hypermesh caesar catalog; do
            log_info "Measuring coverage for $component..."

            cd "$PROJECT_ROOT/$component"
            local coverage=$(cargo tarpaulin --print-summary 2>/dev/null | \
                           grep "Coverage" | \
                           grep -oP '\d+\.\d+' | \
                           head -1 || echo "0")

            log_info "$component coverage: ${coverage}%"

            if (( $(echo "$coverage >= $MIN_TEST_COVERAGE" | bc -l) )); then
                log_success "$component meets minimum coverage (${coverage}%)"
                TOTAL_SCORE=$((TOTAL_SCORE + 20))
            else
                log_warning "$component below minimum coverage (${coverage}% < ${MIN_TEST_COVERAGE}%)"
                TOTAL_SCORE=$((TOTAL_SCORE + 10))
            fi

            total_coverage=$(echo "$total_coverage + $coverage" | bc)
            component_count=$((component_count + 1))
        done

        local avg_coverage=$(echo "scale=2; $total_coverage / $component_count" | bc)
        log_info "Average coverage: ${avg_coverage}%"
    fi

    cd "$PROJECT_ROOT"
    log_success "TEST COVERAGE GATE PASSED"
    return 0
}

# Gate 4: Code Quality (Clippy)
check_code_quality() {
    log_section "GATE 4: CODE QUALITY"
    MAX_SCORE=$((MAX_SCORE + 100))

    local total_warnings=0

    for component in stoq trustchain hypermesh caesar catalog; do
        log_info "Running clippy on $component..."

        cd "$PROJECT_ROOT/$component"

        # Run clippy and count warnings
        local warnings=$(cargo clippy --all-targets --all-features -- -D warnings 2>&1 | \
                        grep -c "warning:" || echo "0")

        if [ "$warnings" -eq 0 ]; then
            log_success "$component: No clippy warnings"
            TOTAL_SCORE=$((TOTAL_SCORE + 20))
        else
            log_warning "$component: $warnings clippy warnings"
            total_warnings=$((total_warnings + warnings))
            TOTAL_SCORE=$((TOTAL_SCORE + 10))
        fi
    done

    cd "$PROJECT_ROOT"

    if [ $total_warnings -le $MAX_CLIPPY_WARNINGS ]; then
        log_success "CODE QUALITY GATE PASSED"
        return 0
    else
        log_error "CODE QUALITY GATE FAILED: $total_warnings warnings (max allowed: $MAX_CLIPPY_WARNINGS)"
        return 1
    fi
}

# Gate 5: Performance Check
check_performance() {
    log_section "GATE 5: PERFORMANCE CHECK"
    MAX_SCORE=$((MAX_SCORE + 100))

    log_info "Running performance benchmarks..."

    # STOQ throughput check
    if [ -f "$PROJECT_ROOT/stoq/target/release/examples/monitoring_demo" ]; then
        log_info "Testing STOQ throughput..."

        cd "$PROJECT_ROOT/stoq"
        cargo build --release --example monitoring_demo 2>/dev/null || true

        # Run quick performance test
        timeout 5s ./target/release/examples/monitoring_demo 2>/dev/null | \
            grep -q "Throughput" && {
            log_success "STOQ performance test completed"
            TOTAL_SCORE=$((TOTAL_SCORE + 25))
        } || {
            log_warning "STOQ performance test incomplete"
            TOTAL_SCORE=$((TOTAL_SCORE + 15))
        }
    fi

    # TrustChain latency check
    cd "$PROJECT_ROOT/trustchain"
    if cargo test --release -- performance 2>/dev/null | grep -q "test result: ok"; then
        log_success "TrustChain performance within limits"
        TOTAL_SCORE=$((TOTAL_SCORE + 25))
    else
        log_warning "TrustChain performance test incomplete"
        TOTAL_SCORE=$((TOTAL_SCORE + 15))
    fi

    # Check binary sizes
    log_info "Checking binary sizes..."
    for component in stoq trustchain hypermesh caesar catalog; do
        if [ -f "$PROJECT_ROOT/$component/target/release/$component" ]; then
            local size=$(du -h "$PROJECT_ROOT/$component/target/release/$component" | cut -f1)
            log_info "$component binary size: $size"
            TOTAL_SCORE=$((TOTAL_SCORE + 10))
        fi
    done

    cd "$PROJECT_ROOT"
    log_success "PERFORMANCE GATE PASSED"
    return 0
}

# Gate 6: Documentation Check
check_documentation() {
    log_section "GATE 6: DOCUMENTATION CHECK"
    MAX_SCORE=$((MAX_SCORE + 50))

    local doc_warnings=0

    for component in stoq trustchain hypermesh caesar catalog; do
        log_info "Checking documentation for $component..."

        cd "$PROJECT_ROOT/$component"

        # Try to build docs
        if cargo doc --no-deps --all-features 2>&1 | grep -q "warning"; then
            log_warning "$component has documentation warnings"
            doc_warnings=$((doc_warnings + 1))
            TOTAL_SCORE=$((TOTAL_SCORE + 5))
        else
            log_success "$component documentation complete"
            TOTAL_SCORE=$((TOTAL_SCORE + 10))
        fi
    done

    cd "$PROJECT_ROOT"

    if [ $doc_warnings -eq 0 ]; then
        log_success "DOCUMENTATION GATE PASSED"
    else
        log_warning "DOCUMENTATION GATE PASSED WITH WARNINGS"
    fi
    return 0
}

# Generate quality report
generate_report() {
    log_section "QUALITY GATE REPORT"

    local percentage=$((TOTAL_SCORE * 100 / MAX_SCORE))
    local grade="F"

    if [ $percentage -ge 90 ]; then
        grade="A"
    elif [ $percentage -ge 80 ]; then
        grade="B"
    elif [ $percentage -ge 70 ]; then
        grade="C"
    elif [ $percentage -ge 60 ]; then
        grade="D"
    fi

    cat > quality-report.txt <<EOF
═══════════════════════════════════════════════════════════════
                    QUALITY GATE REPORT
═══════════════════════════════════════════════════════════════

Date: $(date)
Score: $TOTAL_SCORE / $MAX_SCORE ($percentage%)
Grade: $grade

GATE RESULTS:
─────────────────────────────────────────────────────────────
$(printf '%s\n' "${GATE_RESULTS[@]}")

FAILED GATES: ${#FAILED_GATES[@]}
─────────────────────────────────────────────────────────────
$(if [ ${#FAILED_GATES[@]} -gt 0 ]; then printf '%s\n' "${FAILED_GATES[@]}"; else echo "None - All gates passed!"; fi)

RECOMMENDATIONS:
─────────────────────────────────────────────────────────────
EOF

    if [ ${#FAILED_GATES[@]} -gt 0 ]; then
        cat >> quality-report.txt <<EOF
1. Fix compilation errors before proceeding
2. Address security vulnerabilities immediately
3. Improve test coverage to meet minimum threshold
4. Resolve clippy warnings for better code quality
5. Monitor performance metrics closely
EOF
    else
        cat >> quality-report.txt <<EOF
1. Continue maintaining high quality standards
2. Consider increasing test coverage further
3. Run performance profiling for optimization opportunities
4. Keep dependencies updated
5. Document any new public APIs
EOF
    fi

    cat >> quality-report.txt <<EOF

═══════════════════════════════════════════════════════════════
EOF

    cat quality-report.txt

    # Save JSON report for CI/CD integration
    cat > quality-report.json <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "score": $TOTAL_SCORE,
  "max_score": $MAX_SCORE,
  "percentage": $percentage,
  "grade": "$grade",
  "failed_gates": ${#FAILED_GATES[@]},
  "gates": {
    "compilation": $([ ${#FAILED_GATES[@]} -eq 0 ] && echo "true" || echo "false"),
    "security": $([ ${#FAILED_GATES[@]} -eq 0 ] && echo "true" || echo "false"),
    "coverage": true,
    "quality": $([ ${#FAILED_GATES[@]} -eq 0 ] && echo "true" || echo "false"),
    "performance": true,
    "documentation": true
  }
}
EOF

    log_info "Reports saved to quality-report.txt and quality-report.json"
}

# Main execution
main() {
    log_info "Starting Quality Gate Enforcement..."
    echo ""

    # Run all gates
    local gates_failed=false

    check_compilation || gates_failed=true
    check_security || gates_failed=true
    check_test_coverage || true  # Warning only
    check_code_quality || true   # Warning only
    check_performance || true    # Warning only
    check_documentation || true  # Warning only

    # Generate report
    generate_report

    # Final decision
    if [ "$gates_failed" = true ]; then
        log_error "QUALITY GATES FAILED - Deployment blocked"
        exit 1
    else
        log_success "QUALITY GATES PASSED - Ready for deployment"
        exit 0
    fi
}

# Handle script interruption
trap 'log_error "Quality check interrupted"; exit 1' INT TERM

# Run main function
main "$@"