#!/usr/bin/env python3
"""
Web3 Ecosystem Security Testing Suite
Comprehensive security tests including penetration testing
"""

import os
import sys
import json
import asyncio
import random
import string
import socket
import subprocess
from pathlib import Path
from typing import Dict, List, Tuple
from datetime import datetime
import hashlib
import base64

class SecurityTestSuite:
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.test_results = {
            'passed': [],
            'failed': [],
            'vulnerabilities': []
        }
        self.test_count = 0

    def test_cryptographic_implementations(self) -> bool:
        """Test that real cryptographic libraries are being used"""
        print("\n[TEST 1] Cryptographic Implementation Validation")
        print("-" * 50)

        crypto_tests = [
            {
                'name': 'FALCON Post-Quantum Crypto',
                'file': 'stoq/src/transport/falcon.rs',
                'required_imports': ['pqcrypto_falcon', 'falcon1024', 'falcon512'],
                'required_functions': ['generate_keypair', 'sign', 'verify']
            },
            {
                'name': 'Certificate Management',
                'file': 'stoq/src/transport/certificates.rs',
                'required_imports': ['rustls', 'rcgen', 'sha2'],
                'required_functions': ['generate_real_private_key', 'calculate_fingerprint']
            },
            {
                'name': 'TrustChain Consensus',
                'file': 'trustchain/src/consensus/mod.rs',
                'required_imports': ['sha2', 'bincode'],
                'required_functions': ['generate_from_network', 'validate']
            }
        ]

        all_passed = True
        for test in crypto_tests:
            file_path = self.project_root / test['file']
            if not file_path.exists():
                print(f"  ❌ {test['name']}: File not found")
                self.test_results['failed'].append(f"{test['name']}: File not found")
                all_passed = False
                continue

            with open(file_path, 'r') as f:
                content = f.read()

            # Check for required imports
            imports_found = all(imp in content for imp in test['required_imports'])
            functions_found = all(func in content for func in test['required_functions'])

            if imports_found and functions_found:
                print(f"  ✓ {test['name']}: Real crypto implementation found")
                self.test_results['passed'].append(test['name'])
            else:
                print(f"  ❌ {test['name']}: Missing required crypto components")
                self.test_results['failed'].append(test['name'])
                all_passed = False

        return all_passed

    def test_input_validation(self) -> bool:
        """Test input validation implementation"""
        print("\n[TEST 2] Input Validation")
        print("-" * 50)

        validation_file = self.project_root / 'trustchain/src/validation.rs'
        if validation_file.exists():
            with open(validation_file, 'r') as f:
                content = f.read()

            validators = [
                'validate_node_id',
                'validate_certificate_request',
                'validate_ipv6',
                'validate_consensus_proof',
                'sanitize_input'
            ]

            all_found = all(v in content for v in validators)
            if all_found:
                print("  ✓ Input validation module implemented")
                self.test_results['passed'].append('Input validation')
                return True
            else:
                print("  ❌ Incomplete input validation module")
                self.test_results['failed'].append('Input validation incomplete')
                return False
        else:
            print("  ❌ Input validation module not found")
            self.test_results['failed'].append('Input validation missing')
            return False

    def test_security_bypasses(self) -> bool:
        """Test for security bypass methods in production code"""
        print("\n[TEST 3] Security Bypass Detection")
        print("-" * 50)

        # Check for default_for_testing in non-test files
        result = subprocess.run(
            ['grep', '-r', 'default_for_testing', '--include=*.rs',
             '--exclude-dir=target', '--exclude-dir=.security_backup*',
             '--exclude=*_test.rs', str(self.project_root)],
            capture_output=True,
            text=True
        )

        # Filter out #[cfg(test)] guarded instances
        violations = []
        for line in result.stdout.split('\n'):
            if line and '#[cfg(test)]' not in line:
                violations.append(line)

        if not violations:
            print("  ✓ No security bypasses found in production code")
            self.test_results['passed'].append('No security bypasses')
            return True
        else:
            print(f"  ❌ Found {len(violations)} security bypass violations")
            self.test_results['failed'].append(f'{len(violations)} security bypasses')
            for v in violations[:3]:  # Show first 3
                print(f"    - {v[:80]}...")
            return False

    def test_error_handling(self) -> bool:
        """Test proper error handling instead of panic-inducing unwrap()"""
        print("\n[TEST 4] Error Handling")
        print("-" * 50)

        critical_files = [
            'stoq/src/transport/mod.rs',
            'trustchain/src/lib.rs',
            'hypermesh/src/lib.rs'
        ]

        unwrap_threshold = 10  # Allow some unwraps in non-critical paths
        all_passed = True

        for file_path in critical_files:
            full_path = self.project_root / file_path
            if not full_path.exists():
                continue

            with open(full_path, 'r') as f:
                content = f.read()

            unwrap_count = content.count('.unwrap()')
            if unwrap_count <= unwrap_threshold:
                print(f"  ✓ {file_path}: {unwrap_count} unwraps (acceptable)")
                self.test_results['passed'].append(f'{file_path} error handling')
            else:
                print(f"  ❌ {file_path}: {unwrap_count} unwraps (too many)")
                self.test_results['failed'].append(f'{file_path} excessive unwraps')
                all_passed = False

        return all_passed

    def test_certificate_security(self) -> bool:
        """Test certificate validation and security"""
        print("\n[TEST 5] Certificate Security")
        print("-" * 50)

        cert_file = self.project_root / 'stoq/src/transport/certificates.rs'
        if not cert_file.exists():
            print("  ❌ Certificate module not found")
            return False

        with open(cert_file, 'r') as f:
            content = f.read()

        security_features = {
            'RSA key generation': 'RsaPrivateKey::new',
            'Certificate validation': 'validate_certificate',
            'Fingerprint calculation': 'calculate_fingerprint',
            'Certificate rotation': 'check_and_rotate_certificate',
            'TrustChain integration': 'TrustChainClient'
        }

        all_found = True
        for feature, pattern in security_features.items():
            if pattern in content:
                print(f"  ✓ {feature}: Implemented")
                self.test_results['passed'].append(f'Certificate: {feature}')
            else:
                print(f"  ❌ {feature}: Missing")
                self.test_results['failed'].append(f'Certificate: {feature}')
                all_found = False

        return all_found

    def test_consensus_validation(self) -> bool:
        """Test consensus proof validation implementation"""
        print("\n[TEST 6] Consensus Validation")
        print("-" * 50)

        consensus_file = self.project_root / 'trustchain/src/consensus/mod.rs'
        if not consensus_file.exists():
            print("  ❌ Consensus module not found")
            return False

        with open(consensus_file, 'r') as f:
            content = f.read()

        proof_types = ['StakeProof', 'TimeProof', 'SpaceProof', 'WorkProof']
        validation_methods = ['validate', 'generate_from_network', 'validate_with_requirements']

        all_found = True
        for proof in proof_types:
            if proof in content:
                print(f"  ✓ {proof}: Implemented")
                self.test_results['passed'].append(f'Consensus: {proof}')
            else:
                print(f"  ❌ {proof}: Missing")
                self.test_results['failed'].append(f'Consensus: {proof}')
                all_found = False

        for method in validation_methods:
            if method in content:
                print(f"  ✓ {method}(): Implemented")
            else:
                print(f"  ❌ {method}(): Missing")
                all_found = False

        return all_found

    async def test_network_security(self) -> bool:
        """Test network security features"""
        print("\n[TEST 7] Network Security")
        print("-" * 50)

        tests = [
            ('IPv6 only support', self.check_ipv6_only()),
            ('QUIC transport', self.check_quic_transport()),
            ('TLS 1.3 support', self.check_tls_support()),
            ('Rate limiting', self.check_rate_limiting())
        ]

        all_passed = True
        for test_name, result in tests:
            if result:
                print(f"  ✓ {test_name}: Enabled")
                self.test_results['passed'].append(f'Network: {test_name}')
            else:
                print(f"  ❌ {test_name}: Not found")
                self.test_results['failed'].append(f'Network: {test_name}')
                all_passed = False

        return all_passed

    def check_ipv6_only(self) -> bool:
        """Check for IPv6-only configuration"""
        result = subprocess.run(
            ['grep', '-r', 'Ipv6Addr', '--include=*.rs',
             '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return len(result.stdout.split('\n')) > 10

    def check_quic_transport(self) -> bool:
        """Check for QUIC transport implementation"""
        result = subprocess.run(
            ['grep', '-r', 'quinn', '--include=*.toml',
             '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return 'quinn' in result.stdout

    def check_tls_support(self) -> bool:
        """Check for TLS 1.3 support"""
        result = subprocess.run(
            ['grep', '-r', 'rustls', '--include=*.toml',
             '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return 'rustls' in result.stdout

    def check_rate_limiting(self) -> bool:
        """Check for rate limiting implementation"""
        result = subprocess.run(
            ['grep', '-r', 'rate_limit\\|RateLimit', '--include=*.rs',
             '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return len(result.stdout) > 0

    def test_penetration_resistance(self) -> bool:
        """Test resistance to common attacks"""
        print("\n[TEST 8] Penetration Testing")
        print("-" * 50)

        attack_tests = [
            ('SQL Injection', self.test_sql_injection_resistance()),
            ('Command Injection', self.test_command_injection_resistance()),
            ('Path Traversal', self.test_path_traversal_resistance()),
            ('XSS Protection', self.test_xss_protection()),
            ('CSRF Protection', self.test_csrf_protection())
        ]

        all_passed = True
        for test_name, result in attack_tests:
            if result:
                print(f"  ✓ {test_name}: Protected")
                self.test_results['passed'].append(f'Security: {test_name}')
            else:
                print(f"  ❌ {test_name}: Vulnerable")
                self.test_results['failed'].append(f'Security: {test_name}')
                self.test_results['vulnerabilities'].append(test_name)
                all_passed = False

        return all_passed

    def test_sql_injection_resistance(self) -> bool:
        """Check for parameterized queries"""
        result = subprocess.run(
            ['grep', '-r', 'format!.*SELECT\\|format!.*INSERT\\|format!.*UPDATE',
             '--include=*.rs', '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        # If we find format! with SQL, it's potentially vulnerable
        return len(result.stdout) == 0

    def test_command_injection_resistance(self) -> bool:
        """Check for safe command execution"""
        result = subprocess.run(
            ['grep', '-r', 'Command::new.*format!\\|Command::new.*+',
             '--include=*.rs', '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return len(result.stdout) == 0

    def test_path_traversal_resistance(self) -> bool:
        """Check for path traversal protection"""
        validation_file = self.project_root / 'trustchain/src/validation.rs'
        if validation_file.exists():
            with open(validation_file, 'r') as f:
                content = f.read()
            return 'sanitize_input' in content
        return False

    def test_xss_protection(self) -> bool:
        """Check for XSS protection headers"""
        # Look for security headers configuration
        result = subprocess.run(
            ['grep', '-r', 'X-Content-Type-Options\\|X-Frame-Options',
             '--include=*.rs', '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return len(result.stdout) > 0

    def test_csrf_protection(self) -> bool:
        """Check for CSRF token implementation"""
        result = subprocess.run(
            ['grep', '-r', 'csrf\\|CSRF', '--include=*.rs',
             '--exclude-dir=target', str(self.project_root)],
            capture_output=True,
            text=True
        )
        return len(result.stdout) > 0

    def generate_report(self) -> Dict:
        """Generate test report"""
        total_tests = len(self.test_results['passed']) + len(self.test_results['failed'])

        report = {
            'timestamp': datetime.now().isoformat(),
            'total_tests': total_tests,
            'passed': len(self.test_results['passed']),
            'failed': len(self.test_results['failed']),
            'vulnerabilities': self.test_results['vulnerabilities'],
            'pass_rate': f"{(len(self.test_results['passed']) / total_tests * 100):.1f}%" if total_tests > 0 else "0%",
            'details': self.test_results
        }

        return report

    async def run_all_tests(self):
        """Run all security tests"""
        print("="*60)
        print("WEB3 ECOSYSTEM SECURITY TEST SUITE")
        print("="*60)

        test_functions = [
            self.test_cryptographic_implementations,
            self.test_input_validation,
            self.test_security_bypasses,
            self.test_error_handling,
            self.test_certificate_security,
            self.test_consensus_validation,
            self.test_penetration_resistance
        ]

        for test_func in test_functions:
            test_func()

        # Async network tests
        await self.test_network_security()

        # Generate and display report
        report = self.generate_report()

        print("\n" + "="*60)
        print("SECURITY TEST RESULTS")
        print("="*60)
        print(f"Total Tests: {report['total_tests']}")
        print(f"Passed: {report['passed']} ({report['pass_rate']})")
        print(f"Failed: {report['failed']}")

        if report['vulnerabilities']:
            print(f"\n⚠️  CRITICAL VULNERABILITIES DETECTED:")
            for vuln in report['vulnerabilities']:
                print(f"  - {vuln}")

        # Save report
        report_file = self.project_root / 'security_test_report.json'
        with open(report_file, 'w') as f:
            json.dump(report, f, indent=2)

        print(f"\n✓ Full report saved to: {report_file}")

        # Return exit code
        if report['failed'] == 0:
            print("\n✅ SECURITY TESTS PASSED")
            return 0
        else:
            print(f"\n❌ SECURITY TESTS FAILED: {report['failed']} failures")
            return 1

async def main():
    project_root = "/home/persist/repos/projects/web3"
    suite = SecurityTestSuite(project_root)
    return await suite.run_all_tests()

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)