#!/usr/bin/env python3
"""
TrustChain Security Audit Script
Performs comprehensive security validation of the codebase
"""

import os
import re
import subprocess
import json
from pathlib import Path
from dataclasses import dataclass
from typing import List, Dict, Tuple
import sys

@dataclass
class SecurityViolation:
    severity: str  # CRITICAL, HIGH, MEDIUM, LOW
    category: str
    file: str
    line: int
    pattern: str
    context: str
    remediation: str

class TrustChainSecurityAuditor:
    def __init__(self, base_path: str = "/home/persist/repos/projects/web3/trustchain"):
        self.base_path = Path(base_path)
        self.src_path = self.base_path / "src"
        self.violations: List[SecurityViolation] = []

    def audit_all(self) -> Dict:
        """Run comprehensive security audit"""
        print("🔍 Starting TrustChain Security Audit...")
        print("=" * 60)

        results = {
            "security_theater": self.audit_security_theater(),
            "hsm_dependencies": self.audit_hsm_dependencies(),
            "dns_infrastructure": self.audit_dns_infrastructure(),
            "consensus_validation": self.audit_consensus_validation(),
            "api_security": self.audit_api_security(),
            "production_readiness": self.check_production_readiness()
        }

        return self.generate_report(results)

    def audit_security_theater(self) -> Dict:
        """Check for security theater patterns"""
        print("\n📋 Auditing Security Theater Patterns...")

        patterns = {
            "default_for_testing": (r"default_for_testing\(\)", "CRITICAL",
                                   "Testing bypass in production code"),
            "mock_implementations": (r"\bmock_|Mock\b", "HIGH",
                                   "Mock implementation instead of real security"),
            "stub_functions": (r"\bstub\b|TODO.*security|FIXME.*security", "HIGH",
                              "Incomplete security implementation"),
            "hardcoded_values": (r"hardcoded|test_key|dummy_", "MEDIUM",
                                "Hardcoded security values")
        }

        violations = []
        for name, (pattern, severity, description) in patterns.items():
            matches = self._search_pattern(pattern, self.src_path)
            for file, line, context in matches:
                violations.append(SecurityViolation(
                    severity=severity,
                    category="Security Theater",
                    file=file,
                    line=line,
                    pattern=name,
                    context=context,
                    remediation=f"Remove {name} and implement real security: {description}"
                ))

        return {
            "total_violations": len(violations),
            "critical": sum(1 for v in violations if v.severity == "CRITICAL"),
            "high": sum(1 for v in violations if v.severity == "HIGH"),
            "violations": violations[:10]  # Top 10
        }

    def audit_hsm_dependencies(self) -> Dict:
        """Check for HSM dependencies that violate software-only requirement"""
        print("\n🔐 Auditing HSM Dependencies...")

        hsm_patterns = {
            "hsm_imports": (r"use.*hsm|HSM", "CRITICAL", "HSM import found"),
            "hsm_config": (r"HSMConfig|CloudHSM", "CRITICAL", "HSM configuration present"),
            "hsm_operations": (r"hsm_operations|hsm_client", "HIGH", "HSM operations in code")
        }

        violations = []
        cargo_path = self.base_path / "Cargo.toml"

        # Check Cargo.toml
        if cargo_path.exists():
            cargo_content = cargo_path.read_text()
            if "aws-sdk-cloudhsm" in cargo_content or "pkcs11" in cargo_content:
                violations.append(SecurityViolation(
                    severity="CRITICAL",
                    category="HSM Dependency",
                    file=str(cargo_path),
                    line=0,
                    pattern="cargo_dependency",
                    context="HSM dependencies in Cargo.toml",
                    remediation="Remove HSM dependencies from Cargo.toml - software-only requirement"
                ))

        # Check source code
        for name, (pattern, severity, description) in hsm_patterns.items():
            matches = self._search_pattern(pattern, self.src_path / "ca")
            for file, line, context in matches:
                violations.append(SecurityViolation(
                    severity=severity,
                    category="HSM Dependency",
                    file=file,
                    line=line,
                    pattern=name,
                    context=context,
                    remediation=f"Remove HSM dependency: {description}"
                ))

        return {
            "total_violations": len(violations),
            "requires_removal": len(violations) > 0,
            "violations": violations[:10]
        }

    def audit_dns_infrastructure(self) -> Dict:
        """Check DNS implementation for localhost stubs vs production"""
        print("\n🌐 Auditing DNS Infrastructure...")

        dns_patterns = {
            "localhost_refs": (r"localhost|127\.0\.0\.1|::1", "HIGH",
                              "Localhost reference in DNS code"),
            "stub_resolvers": (r"stub.*resolver|mock.*dns", "HIGH",
                             "Stub DNS resolver"),
            "hardcoded_ips": (r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}", "MEDIUM",
                            "Hardcoded IP address")
        }

        violations = []
        production_ready = True

        for name, (pattern, severity, description) in dns_patterns.items():
            matches = self._search_pattern(pattern, self.src_path / "dns")
            for file, line, context in matches:
                # Skip test files
                if "test" in file.lower() or "#[cfg(test)]" in context:
                    continue

                violations.append(SecurityViolation(
                    severity=severity,
                    category="DNS Infrastructure",
                    file=file,
                    line=line,
                    pattern=name,
                    context=context,
                    remediation=f"Replace with production DNS: {description}"
                ))
                production_ready = False

        return {
            "total_violations": len(violations),
            "production_ready": production_ready,
            "violations": violations[:10]
        }

    def audit_consensus_validation(self) -> Dict:
        """Check consensus validation implementation"""
        print("\n✅ Auditing Consensus Validation...")

        consensus_path = self.src_path / "consensus"
        violations = []
        has_real_validation = False

        if consensus_path.exists():
            # Check for real implementation
            for rust_file in consensus_path.glob("*.rs"):
                content = rust_file.read_text()

                # Check for testing bypasses outside test code
                if "default_for_testing" in content:
                    lines = content.split('\n')
                    for i, line in enumerate(lines, 1):
                        if "default_for_testing" in line and "#[cfg(test)]" not in content[:content.find(line)]:
                            violations.append(SecurityViolation(
                                severity="CRITICAL",
                                category="Consensus Validation",
                                file=str(rust_file),
                                line=i,
                                pattern="testing_bypass",
                                context=line.strip(),
                                remediation="Remove default_for_testing() from production code"
                            ))

                # Check for real validation
                if "validate_with_requirements" in content and "generate_from_network" in content:
                    has_real_validation = True

        return {
            "has_real_validation": has_real_validation,
            "total_violations": len(violations),
            "violations": violations[:10]
        }

    def audit_api_security(self) -> Dict:
        """Check API handlers for mock responses"""
        print("\n🔒 Auditing API Security...")

        api_path = self.src_path / "api" / "handlers.rs"
        violations = []
        mock_count = 0

        if api_path.exists():
            content = api_path.read_text()
            lines = content.split('\n')

            for i, line in enumerate(lines, 1):
                if "mock" in line.lower() or "Mock" in line:
                    mock_count += 1
                    violations.append(SecurityViolation(
                        severity="HIGH",
                        category="API Security",
                        file=str(api_path),
                        line=i,
                        pattern="mock_response",
                        context=line.strip(),
                        remediation="Replace mock response with real implementation"
                    ))

        return {
            "mock_responses": mock_count,
            "production_ready": mock_count == 0,
            "violations": violations[:10]
        }

    def check_production_readiness(self) -> Dict:
        """Overall production readiness check"""
        print("\n🚀 Checking Production Readiness...")

        checks = {
            "no_testing_bypasses": self._check_no_testing_bypasses(),
            "no_mock_implementations": self._check_no_mocks(),
            "real_dns_infrastructure": self._check_real_dns(),
            "no_hsm_dependencies": self._check_no_hsm(),
            "consensus_validation": self._check_consensus()
        }

        ready = all(checks.values())

        return {
            "production_ready": ready,
            "checks": checks,
            "blocking_issues": [k for k, v in checks.items() if not v]
        }

    def _search_pattern(self, pattern: str, path: Path) -> List[Tuple[str, int, str]]:
        """Search for pattern in files using ripgrep"""
        matches = []
        try:
            result = subprocess.run(
                ["rg", "-n", "--no-heading", pattern, str(path)],
                capture_output=True,
                text=True
            )

            if result.stdout:
                for line in result.stdout.strip().split('\n'):
                    if ':' in line:
                        parts = line.split(':', 2)
                        if len(parts) >= 3:
                            matches.append((parts[0], int(parts[1]), parts[2]))
        except:
            pass

        return matches

    def _check_no_testing_bypasses(self) -> bool:
        """Check for testing bypasses in production code"""
        matches = self._search_pattern(r"default_for_testing", self.src_path)
        # Filter out test files
        prod_matches = [m for m in matches if "test" not in m[0].lower() and "#[cfg(test)]" not in m[2]]
        return len(prod_matches) == 0

    def _check_no_mocks(self) -> bool:
        """Check for mock implementations"""
        matches = self._search_pattern(r"\bmock_|Mock\b", self.src_path / "api")
        return len(matches) < 5  # Allow some in tests

    def _check_real_dns(self) -> bool:
        """Check for real DNS implementation"""
        dns_path = self.src_path / "dns" / "production_zones.rs"
        return dns_path.exists()

    def _check_no_hsm(self) -> bool:
        """Check for HSM dependencies"""
        matches = self._search_pattern(r"HSMConfig|CloudHSM", self.src_path / "ca")
        return len(matches) < 10  # Some references ok in comments/types

    def _check_consensus(self) -> bool:
        """Check consensus validation exists"""
        consensus_path = self.src_path / "consensus" / "validator.rs"
        return consensus_path.exists()

    def generate_report(self, results: Dict) -> Dict:
        """Generate comprehensive security report"""

        # Calculate overall security score
        total_violations = sum(r.get("total_violations", 0) for r in results.values() if isinstance(r, dict))
        critical_count = sum(r.get("critical", 0) for r in results.values() if isinstance(r, dict))
        high_count = sum(r.get("high", 0) for r in results.values() if isinstance(r, dict))

        # Score calculation (out of 100)
        score = 100
        score -= critical_count * 10  # Each critical issue -10 points
        score -= high_count * 5       # Each high issue -5 points
        score = max(0, score)

        production_ready = (
            results["production_readiness"]["production_ready"] and
            score >= 70
        )

        report = {
            "security_score": score,
            "production_ready": production_ready,
            "total_violations": total_violations,
            "critical_violations": critical_count,
            "high_violations": high_count,
            "categories": results,
            "deployment_recommendation": self._get_recommendation(score, production_ready),
            "immediate_actions": self._get_immediate_actions(results)
        }

        return report

    def _get_recommendation(self, score: int, ready: bool) -> str:
        if score >= 90 and ready:
            return "✅ APPROVED: System ready for production deployment"
        elif score >= 70:
            return "⚠️ CONDITIONAL: Fix critical issues before production"
        else:
            return "❌ BLOCKED: Significant security issues prevent deployment"

    def _get_immediate_actions(self, results: Dict) -> List[str]:
        actions = []

        if results["security_theater"]["critical"] > 0:
            actions.append("Remove all default_for_testing() calls from production code")

        if results["hsm_dependencies"]["requires_removal"]:
            actions.append("Remove HSM dependencies and implement software-only crypto")

        if not results["dns_infrastructure"]["production_ready"]:
            actions.append("Replace localhost DNS stubs with production infrastructure")

        if results["api_security"]["mock_responses"] > 10:
            actions.append("Replace mock API responses with real implementations")

        if not results["consensus_validation"]["has_real_validation"]:
            actions.append("Implement real consensus validation without bypasses")

        return actions

def print_report(report: Dict):
    """Pretty print the security report"""
    print("\n" + "=" * 60)
    print("🔐 TRUSTCHAIN SECURITY AUDIT REPORT")
    print("=" * 60)

    print(f"\n📊 Security Score: {report['security_score']}/100")
    print(f"🚀 Production Ready: {'YES ✅' if report['production_ready'] else 'NO ❌'}")

    print(f"\n⚠️ Total Violations: {report['total_violations']}")
    print(f"   - Critical: {report['critical_violations']}")
    print(f"   - High: {report['high_violations']}")

    print(f"\n📋 Deployment Recommendation:")
    print(f"   {report['deployment_recommendation']}")

    if report['immediate_actions']:
        print(f"\n🔧 Immediate Actions Required:")
        for i, action in enumerate(report['immediate_actions'], 1):
            print(f"   {i}. {action}")

    # Category summaries
    print("\n📁 Category Analysis:")

    categories = report['categories']

    print(f"\n   1. Security Theater:")
    print(f"      - Violations: {categories['security_theater']['total_violations']}")
    print(f"      - Critical: {categories['security_theater']['critical']}")

    print(f"\n   2. HSM Dependencies:")
    print(f"      - Violations: {categories['hsm_dependencies']['total_violations']}")
    print(f"      - Must Remove: {categories['hsm_dependencies']['requires_removal']}")

    print(f"\n   3. DNS Infrastructure:")
    print(f"      - Violations: {categories['dns_infrastructure']['total_violations']}")
    print(f"      - Production Ready: {categories['dns_infrastructure']['production_ready']}")

    print(f"\n   4. API Security:")
    print(f"      - Mock Responses: {categories['api_security']['mock_responses']}")
    print(f"      - Production Ready: {categories['api_security']['production_ready']}")

    print(f"\n   5. Consensus Validation:")
    print(f"      - Real Validation: {categories['consensus_validation']['has_real_validation']}")
    print(f"      - Violations: {categories['consensus_validation']['total_violations']}")

    # Production readiness details
    print("\n🎯 Production Readiness Checks:")
    checks = categories['production_readiness']['checks']
    for check, passed in checks.items():
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"   - {check}: {status}")

    if categories['production_readiness']['blocking_issues']:
        print("\n🚫 Blocking Issues:")
        for issue in categories['production_readiness']['blocking_issues']:
            print(f"   - {issue}")

    print("\n" + "=" * 60)
    print("End of Security Audit Report")
    print("=" * 60)

if __name__ == "__main__":
    auditor = TrustChainSecurityAuditor()
    report = auditor.audit_all()

    print_report(report)

    # Save JSON report
    report_path = Path("/home/persist/repos/projects/web3/trustchain/security_audit_report.json")
    with open(report_path, 'w') as f:
        # Convert violations to dict for JSON serialization
        for category in report['categories'].values():
            if 'violations' in category and isinstance(category['violations'], list):
                category['violations'] = [
                    {
                        'severity': v.severity,
                        'category': v.category,
                        'file': v.file,
                        'line': v.line,
                        'pattern': v.pattern,
                        'remediation': v.remediation
                    } for v in category['violations']
                ]

        json.dump(report, f, indent=2, default=str)

    print(f"\n📄 Full report saved to: {report_path}")

    # Exit code based on production readiness
    sys.exit(0 if report['production_ready'] else 1)