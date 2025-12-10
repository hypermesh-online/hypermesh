#!/bin/bash
# final_verify.sh - Day 7 comprehensive quality verification

echo "==========================================="
echo "    SPRINT 2.2 FINAL QUALITY REPORT"
echo "==========================================="
echo "Date: $(date)"
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check metric
check_metric() {
    local name=$1
    local current=$2
    local target=$3

    if [ "$current" -le "$target" ]; then
        echo -e "${GREEN}✅${NC} $name: $current (target: ≤$target)"
        return 0
    else
        echo -e "${RED}❌${NC} $name: $current (target: ≤$target)"
        return 1
    fi
}

FAILED_CHECKS=0

echo "=== Code Quality Metrics ==="
echo ""

# 1. Unwrap count
echo "1. UNWRAP ELIMINATION"
UNWRAPS=$(grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | grep -v "mod tests" | wc -l)
check_metric "Production unwraps" "$UNWRAPS" 0 || ((FAILED_CHECKS++))

echo ""
echo "2. TEST SUITE HEALTH"
# Run tests and capture output
TEST_OUTPUT=$(cargo test --lib 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)' | tail -1)
    FAILED=0
    echo -e "${GREEN}✅${NC} Tests: $PASSED passed, 0 failed"
else
    PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)' | tail -1)
    FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | tail -1)
    check_metric "Failed tests" "$FAILED" 0 || ((FAILED_CHECKS++))
    echo "   Total tests: $PASSED passed, $FAILED failed"
fi

echo ""
echo "3. TODO/FIXME CLEANUP"
TODOS=$(grep -rn "TODO\|FIXME" src/ --include="*.rs" | grep -v "test" | wc -l)
CRITICAL_TODOS=$(grep -rn "TODO" src/ --include="*.rs" | grep -v "test" | grep -cE "(Implement|Parse|Extract|Replace)")
check_metric "Critical TODOs" "$CRITICAL_TODOS" 0 || ((FAILED_CHECKS++))
echo "   Total TODOs: $TODOS (including deferred)"

echo ""
echo "4. BUILD WARNINGS"
BUILD_WARNINGS=$(cargo build --lib 2>&1 | grep -c "warning:")
check_metric "Build warnings" "$BUILD_WARNINGS" 5 || ((FAILED_CHECKS++))

echo ""
echo "5. CLIPPY ANALYSIS"
CLIPPY_WARNINGS=$(cargo clippy --lib 2>&1 | grep -c "warning:")
check_metric "Clippy warnings" "$CLIPPY_WARNINGS" 10 || ((FAILED_CHECKS++))

echo ""
echo "6. FORMAT CHECK"
if cargo fmt -- --check 2>&1 | grep -q "Diff"; then
    echo -e "${YELLOW}⚠${NC}  Code formatting: Changes needed"
    ((FAILED_CHECKS++))
else
    echo -e "${GREEN}✅${NC} Code formatting: Compliant"
fi

echo ""
echo "=== Security Audit ==="
if command -v cargo-audit &> /dev/null; then
    VULNS=$(cargo audit 2>&1 | grep -c "Vulnerability")
    check_metric "Security vulnerabilities" "$VULNS" 0 || ((FAILED_CHECKS++))
else
    echo "⚠️  cargo-audit not installed (run: cargo install cargo-audit)"
fi

echo ""
echo "=== Module Summary ==="
for module in ct dns crypto security api; do
    unwraps=$(grep -r "unwrap()" src/$module --include="*.rs" 2>/dev/null | grep -v "test" | wc -l)
    todos=$(grep -r "TODO\|FIXME" src/$module --include="*.rs" 2>/dev/null | grep -v "test" | wc -l)
    if [ "$unwraps" -eq 0 ] && [ "$todos" -eq 0 ]; then
        echo -e "${GREEN}✅${NC} $module: Clean"
    elif [ "$unwraps" -gt 0 ]; then
        echo -e "${RED}⚠${NC}  $module: $unwraps unwraps, $todos TODOs"
    else
        echo -e "${YELLOW}⚠${NC}  $module: $todos TODOs remaining"
    fi
done

echo ""
echo "==========================================="
if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "${GREEN}🎉 SPRINT 2.2 QUALITY GOALS ACHIEVED! 🎉${NC}"
    echo "All production quality metrics pass!"
else
    echo -e "${RED}📊 Quality Issues Remaining: $FAILED_CHECKS${NC}"
    echo "Review failed metrics above for details"
fi
echo "==========================================="

# Generate detailed report file
REPORT_FILE="quality_report_$(date +%Y%m%d_%H%M%S).txt"
echo "Detailed report saved to: $REPORT_FILE"
{
    echo "Sprint 2.2 Quality Report - $(date)"
    echo ""
    echo "Metrics:"
    echo "- Unwraps: $UNWRAPS"
    echo "- Test failures: $FAILED"
    echo "- Critical TODOs: $CRITICAL_TODOS"
    echo "- Build warnings: $BUILD_WARNINGS"
    echo "- Clippy warnings: $CLIPPY_WARNINGS"
    echo ""
    echo "Remaining unwraps by file:"
    grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | cut -d: -f1 | sort | uniq -c | sort -rn
} > "$REPORT_FILE"

exit $FAILED_CHECKS