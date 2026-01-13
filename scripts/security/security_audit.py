#!/usr/bin/env python3
"""
Web3 Ecosystem Security Audit Tool
Comprehensive security analysis to identify and report vulnerabilities
"""

import os
import re
import json
import subprocess
from pathlib import Path
from typing import Dict, List, Tuple
from datetime import datetime
import hashlib

class SecurityAuditor:
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.violations = {
            'critical': [],
            'high': [],
            'medium': [],
            'low': []
        }
        self.stats = {
            'total_files': 0,
            'files_scanned': 0,
            'total_lines': 0,
            'mock_implementations': 0,
            'test_bypasses': 0,
            'unwrap_calls': 0,
            'placeholder_code': 0,
            'missing_validation': 0,
            'hardcoded_secrets': 0,
            'insecure_random': 0,
            'sql_injection': 0,
            'command_injection': 0,
            'path_traversal': 0,
            'unsafe_deserialization': 0
        }

    def scan_rust_file(self, file_path: Path) -> None:
        """Scan a Rust file for security violations"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
                lines = content.split('\n')

            self.stats['files_scanned'] += 1
            self.stats['total_lines'] += len(lines)

            for line_num, line in enumerate(lines, 1):
                # Critical: Mock cryptography
                if re.search(r'(mock|Mock|MOCK).*?(crypto|Crypto|falcon|Falcon|certificate|Certificate)', line, re.IGNORECASE):
                    self.violations['critical'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'MOCK_CRYPTOGRAPHY',
                        'code': line.strip(),
                        'severity': 'CRITICAL',
                        'description': 'Mock cryptographic implementation detected'
                    })
                    self.stats['mock_implementations'] += 1

                # Critical: Test bypass in production
                if 'default_for_testing' in line and not str(file_path).endswith('_test.rs'):
                    self.violations['critical'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'TEST_BYPASS',
                        'code': line.strip(),
                        'severity': 'CRITICAL',
                        'description': 'Test bypass method in production code'
                    })
                    self.stats['test_bypasses'] += 1

                # High: Unwrap without error handling
                if '.unwrap()' in line and not '#[test]' in content[:content.find(line)]:
                    self.violations['high'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'UNWRAP_PANIC',
                        'code': line.strip(),
                        'severity': 'HIGH',
                        'description': 'Unwrap() can cause panic in production'
                    })
                    self.stats['unwrap_calls'] += 1

                # Critical: Placeholder/TODO security code
                if re.search(r'(TODO|FIXME|HACK|XXX).*?(security|Security|crypto|auth|Auth)', line, re.IGNORECASE):
                    self.violations['critical'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'PLACEHOLDER_SECURITY',
                        'code': line.strip(),
                        'severity': 'CRITICAL',
                        'description': 'Placeholder security implementation'
                    })
                    self.stats['placeholder_code'] += 1

                # High: Missing input validation
                if 'parse()' in line and 'expect' not in line and '?' not in line:
                    self.violations['high'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'MISSING_VALIDATION',
                        'code': line.strip(),
                        'severity': 'HIGH',
                        'description': 'Parse without error handling'
                    })
                    self.stats['missing_validation'] += 1

                # Critical: Hardcoded secrets/keys
                if re.search(r'(private_key|secret|password|api_key)\s*=\s*["\'][\w]{8,}["\']', line, re.IGNORECASE):
                    self.violations['critical'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'HARDCODED_SECRET',
                        'code': line.strip()[:50] + '...',
                        'severity': 'CRITICAL',
                        'description': 'Hardcoded secret or key detected'
                    })
                    self.stats['hardcoded_secrets'] += 1

                # High: Insecure random for cryptography
                if 'rand::random' in line and any(x in content for x in ['crypto', 'key', 'nonce', 'salt']):
                    self.violations['high'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'INSECURE_RANDOM',
                        'code': line.strip(),
                        'severity': 'HIGH',
                        'description': 'Non-cryptographic random used for security'
                    })
                    self.stats['insecure_random'] += 1

                # Critical: Command injection risk
                if re.search(r'Command::new.*format!|Command::new.*\+', line):
                    self.violations['critical'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'COMMAND_INJECTION',
                        'code': line.strip(),
                        'severity': 'CRITICAL',
                        'description': 'Potential command injection vulnerability'
                    })
                    self.stats['command_injection'] += 1

                # High: Path traversal risk
                if '../' in line or '..\\' in line:
                    self.violations['high'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'PATH_TRAVERSAL',
                        'code': line.strip(),
                        'severity': 'HIGH',
                        'description': 'Potential path traversal vulnerability'
                    })
                    self.stats['path_traversal'] += 1

                # Medium: Unsafe deserialization
                if 'bincode::deserialize' in line and 'untrusted' not in line.lower():
                    self.violations['medium'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': line_num,
                        'type': 'UNSAFE_DESERIALIZATION',
                        'code': line.strip(),
                        'severity': 'MEDIUM',
                        'description': 'Unsafe deserialization of untrusted data'
                    })
                    self.stats['unsafe_deserialization'] += 1

        except Exception as e:
            print(f"Error scanning {file_path}: {e}")

    def scan_project(self) -> None:
        """Scan entire project for security violations"""
        print(f"Starting security audit of {self.project_root}")

        # Find all Rust files
        rust_files = list(self.project_root.rglob("*.rs"))
        self.stats['total_files'] = len(rust_files)

        # Exclude test, build, and backup directories
        rust_files = [f for f in rust_files if 'target' not in str(f)
                      and 'test' not in f.name
                      and '.security_backup' not in str(f)]

        print(f"Scanning {len(rust_files)} Rust files...")

        for file_path in rust_files:
            self.scan_rust_file(file_path)

    def check_dependencies(self) -> None:
        """Check for vulnerable dependencies"""
        print("\nChecking dependencies for vulnerabilities...")

        cargo_tomls = list(self.project_root.rglob("Cargo.toml"))
        for cargo_path in cargo_tomls:
            if 'target' in str(cargo_path):
                continue

            try:
                # Check for outdated dependencies
                result = subprocess.run(
                    ['cargo', 'outdated', '--manifest-path', str(cargo_path)],
                    capture_output=True,
                    text=True,
                    timeout=30
                )

                if 'outdated' in result.stdout.lower():
                    self.violations['medium'].append({
                        'file': str(cargo_path.relative_to(self.project_root)),
                        'line': 0,
                        'type': 'OUTDATED_DEPENDENCIES',
                        'code': 'Multiple outdated dependencies detected',
                        'severity': 'MEDIUM',
                        'description': 'Outdated dependencies may contain security vulnerabilities'
                    })
            except:
                pass  # cargo-outdated might not be installed

    def analyze_crypto_implementations(self) -> None:
        """Deep analysis of cryptographic implementations"""
        print("\nAnalyzing cryptographic implementations...")

        crypto_files = [
            'stoq/src/transport/falcon.rs',
            'stoq/src/transport/certificates.rs',
            'trustchain/src/crypto/',
            'hypermesh/src/security/'
        ]

        for crypto_path in crypto_files:
            full_path = self.project_root / crypto_path
            if full_path.is_file():
                self.deep_scan_crypto(full_path)
            elif full_path.is_dir():
                for file in full_path.rglob("*.rs"):
                    self.deep_scan_crypto(file)

    def deep_scan_crypto(self, file_path: Path) -> None:
        """Deep scan cryptographic code"""
        try:
            with open(file_path, 'r') as f:
                content = f.read()

            # Check for real vs mock implementations
            if 'pqcrypto' in content or 'ring' in content or 'rustls' in content:
                print(f"  ✓ Real crypto library found in {file_path.name}")
            else:
                if 'crypto' in file_path.name.lower() or 'falcon' in file_path.name.lower():
                    self.violations['critical'].append({
                        'file': str(file_path.relative_to(self.project_root)),
                        'line': 0,
                        'type': 'NO_CRYPTO_LIBRARY',
                        'code': 'File contains crypto code but no real crypto library imports',
                        'severity': 'CRITICAL',
                        'description': 'Cryptographic implementation without proper library'
                    })
        except:
            pass

    def generate_report(self) -> Dict:
        """Generate security audit report"""
        total_violations = sum(len(v) for v in self.violations.values())

        report = {
            'audit_timestamp': datetime.now().isoformat(),
            'project_root': str(self.project_root),
            'summary': {
                'total_violations': total_violations,
                'critical': len(self.violations['critical']),
                'high': len(self.violations['high']),
                'medium': len(self.violations['medium']),
                'low': len(self.violations['low']),
                'files_scanned': self.stats['files_scanned'],
                'total_lines': self.stats['total_lines']
            },
            'violation_types': {
                'mock_implementations': self.stats['mock_implementations'],
                'test_bypasses': self.stats['test_bypasses'],
                'unwrap_calls': self.stats['unwrap_calls'],
                'placeholder_code': self.stats['placeholder_code'],
                'missing_validation': self.stats['missing_validation'],
                'hardcoded_secrets': self.stats['hardcoded_secrets'],
                'insecure_random': self.stats['insecure_random'],
                'command_injection': self.stats['command_injection'],
                'path_traversal': self.stats['path_traversal'],
                'unsafe_deserialization': self.stats['unsafe_deserialization']
            },
            'violations': self.violations,
            'recommendations': self.generate_recommendations()
        }

        return report

    def generate_recommendations(self) -> List[str]:
        """Generate security recommendations based on findings"""
        recommendations = []

        if self.stats['mock_implementations'] > 0:
            recommendations.append("CRITICAL: Replace all mock cryptographic implementations with real libraries (pqcrypto-falcon, ring, rustls)")

        if self.stats['test_bypasses'] > 0:
            recommendations.append("CRITICAL: Remove all default_for_testing() methods from production code paths")

        if self.stats['unwrap_calls'] > 100:
            recommendations.append("HIGH: Replace unwrap() calls with proper error handling using Result<T, E> and ?")

        if self.stats['placeholder_code'] > 0:
            recommendations.append("CRITICAL: Complete all TODO/FIXME security implementations before production")

        if self.stats['hardcoded_secrets'] > 0:
            recommendations.append("CRITICAL: Move all secrets to environment variables or secure key management")

        if self.stats['missing_validation'] > 0:
            recommendations.append("HIGH: Add input validation for all external data sources")

        recommendations.extend([
            "Implement comprehensive input validation framework",
            "Add rate limiting to all public endpoints",
            "Enable security headers (HSTS, CSP, X-Frame-Options)",
            "Implement proper authentication and authorization",
            "Add audit logging for security events",
            "Configure TLS 1.3 with strong cipher suites",
            "Implement certificate pinning for critical connections",
            "Add intrusion detection monitoring",
            "Perform regular dependency updates and security scanning",
            "Implement secure session management"
        ])

        return recommendations

    def print_summary(self, report: Dict) -> None:
        """Print audit summary to console"""
        print("\n" + "="*60)
        print("SECURITY AUDIT REPORT")
        print("="*60)
        print(f"\nAudit completed at: {report['audit_timestamp']}")
        print(f"Files scanned: {report['summary']['files_scanned']}")
        print(f"Total lines analyzed: {report['summary']['total_lines']:,}")

        print(f"\n{'='*40}")
        print("VIOLATION SUMMARY")
        print(f"{'='*40}")
        print(f"CRITICAL: {report['summary']['critical']} violations")
        print(f"HIGH:     {report['summary']['high']} violations")
        print(f"MEDIUM:   {report['summary']['medium']} violations")
        print(f"LOW:      {report['summary']['low']} violations")
        print(f"{'='*40}")
        print(f"TOTAL:    {report['summary']['total_violations']} violations")

        print(f"\n{'='*40}")
        print("VIOLATION TYPES BREAKDOWN")
        print(f"{'='*40}")
        for vtype, count in report['violation_types'].items():
            if count > 0:
                print(f"{vtype:30} {count:5}")

        if report['violations']['critical']:
            print(f"\n{'='*40}")
            print("TOP 5 CRITICAL VIOLATIONS")
            print(f"{'='*40}")
            for v in report['violations']['critical'][:5]:
                print(f"\n{v['type']} - {v['file']}:{v['line']}")
                print(f"  {v['description']}")
                if len(v['code']) < 100:
                    print(f"  Code: {v['code']}")

        print(f"\n{'='*40}")
        print("RECOMMENDATIONS")
        print(f"{'='*40}")
        for i, rec in enumerate(report['recommendations'][:10], 1):
            print(f"{i:2}. {rec}")

def main():
    project_root = "/home/persist/repos/projects/web3"

    auditor = SecurityAuditor(project_root)
    auditor.scan_project()
    auditor.check_dependencies()
    auditor.analyze_crypto_implementations()

    report = auditor.generate_report()
    auditor.print_summary(report)

    # Save detailed report
    report_file = Path(project_root) / "security_audit_report.json"
    with open(report_file, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"\n✓ Detailed report saved to: {report_file}")

    # Return exit code based on critical violations
    if report['summary']['critical'] > 0:
        print(f"\n❌ FAILED: {report['summary']['critical']} critical security violations found")
        return 1
    else:
        print("\n✓ PASSED: No critical security violations found")
        return 0

if __name__ == "__main__":
    exit(main())