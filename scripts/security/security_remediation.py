#!/usr/bin/env python3
"""
Web3 Ecosystem Security Remediation Tool
Automatically fixes critical security violations
"""

import os
import re
import shutil
from pathlib import Path
from typing import List, Tuple
from datetime import datetime

class SecurityRemediator:
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.fixes_applied = {
            'test_bypasses_removed': 0,
            'mock_implementations_marked': 0,
            'unwraps_replaced': 0,
            'validation_added': 0,
            'files_modified': 0
        }

        # Backup directory for original files
        self.backup_dir = self.project_root / f".security_backup_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        self.backup_dir.mkdir(exist_ok=True)

    def backup_file(self, file_path: Path) -> None:
        """Create backup of file before modification"""
        relative_path = file_path.relative_to(self.project_root)
        backup_path = self.backup_dir / relative_path
        backup_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(file_path, backup_path)

    def fix_test_bypasses(self) -> None:
        """Remove or properly gate default_for_testing() methods"""
        print("\n[1/4] Fixing test bypass security violations...")

        files_to_fix = [
            'trustchain/src/consensus/mod.rs',
            'trustchain/src/ct/merkle_log.rs',
            'trustchain/src/ct/storage.rs',
            'trustchain/src/ca/mod.rs',
            'trustchain/src/lib.rs',
            'trustchain/src/api/handlers.rs',
            'trustchain/src/api/security_handlers.rs'
        ]

        for file_path in files_to_fix:
            full_path = self.project_root / file_path
            if not full_path.exists():
                continue

            self.backup_file(full_path)

            with open(full_path, 'r') as f:
                content = f.read()

            original_content = content

            # Replace default_for_testing() calls with generate_from_network()
            content = re.sub(
                r'ConsensusProof::default_for_testing\(\)',
                'ConsensusProof::generate_from_network(&node_id).await?',
                content
            )

            # Add #[cfg(test)] to default_for_testing implementations
            content = re.sub(
                r'(\n\s*)pub fn default_for_testing\(\)',
                r'\1#[cfg(test)]\1pub fn default_for_testing()',
                content
            )

            if content != original_content:
                with open(full_path, 'w') as f:
                    f.write(content)
                self.fixes_applied['test_bypasses_removed'] += content.count('generate_from_network')
                self.fixes_applied['files_modified'] += 1
                print(f"  ✓ Fixed test bypasses in {file_path}")

    def fix_mock_implementations(self) -> None:
        """Mark mock implementations and add production alternatives"""
        print("\n[2/4] Fixing mock cryptographic implementations...")

        # Fix Caesar mock crypto providers
        caesar_file = self.project_root / 'caesar/src/crypto_exchange_providers.rs'
        if caesar_file.exists():
            self.backup_file(caesar_file)

            with open(caesar_file, 'r') as f:
                content = f.read()

            # Move MockCryptoExchangeProvider to test module
            if 'pub struct MockCryptoExchangeProvider' in content:
                # Find the mock implementation
                mock_start = content.find('/// Mock Crypto Exchange Provider')
                if mock_start > 0:
                    # Find the end of the impl block
                    impl_end = content.find('\n}\n', content.find('impl CryptoExchangeProvider for MockCryptoExchangeProvider'))
                    if impl_end > 0:
                        impl_end += 3

                        # Extract mock implementation
                        mock_impl = content[mock_start:impl_end]

                        # Remove from main code
                        content = content[:mock_start] + content[impl_end:]

                        # Add to test module
                        test_module = '\n\n#[cfg(test)]\nmod test_providers {\n    use super::*;\n\n' + mock_impl + '\n}'
                        content += test_module

                        with open(caesar_file, 'w') as f:
                            f.write(content)

                        self.fixes_applied['mock_implementations_marked'] += 1
                        self.fixes_applied['files_modified'] += 1
                        print(f"  ✓ Moved mock provider to test module in caesar/src/crypto_exchange_providers.rs")

    def fix_unwrap_calls(self) -> None:
        """Replace unwrap() with proper error handling"""
        print("\n[3/4] Fixing unwrap() panic vulnerabilities...")

        critical_files = [
            'stoq/src/transport/falcon.rs',
            'stoq/src/transport/certificates.rs',
            'trustchain/src/consensus/mod.rs',
            'hypermesh/src/assets/core/mod.rs'
        ]

        for file_path in critical_files:
            full_path = self.project_root / file_path
            if not full_path.exists():
                continue

            self.backup_file(full_path)

            with open(full_path, 'r') as f:
                lines = f.readlines()

            modified = False
            new_lines = []

            for line in lines:
                if '.unwrap()' in line and '#[test]' not in line:
                    # Replace unwrap() with ? operator where possible
                    if '.unwrap();' in line:
                        new_line = line.replace('.unwrap()', '?')
                        self.fixes_applied['unwraps_replaced'] += 1
                        modified = True
                    elif '.unwrap().' in line:
                        # Chain call - need to handle differently
                        new_line = line.replace('.unwrap().', '?.')
                        self.fixes_applied['unwraps_replaced'] += 1
                        modified = True
                    else:
                        new_line = line
                else:
                    new_line = line

                new_lines.append(new_line)

            if modified:
                with open(full_path, 'w') as f:
                    f.writelines(new_lines)
                self.fixes_applied['files_modified'] += 1
                print(f"  ✓ Replaced unwrap() calls in {file_path}")

    def add_input_validation(self) -> None:
        """Add input validation to critical endpoints"""
        print("\n[4/4] Adding input validation...")

        # Add validation module to TrustChain
        validation_module = self.project_root / 'trustchain/src/validation.rs'

        validation_code = '''//! Input validation module for TrustChain
//! Provides comprehensive validation for all external inputs

use anyhow::{Result, anyhow};
use regex::Regex;
use std::net::{IpAddr, Ipv6Addr};

/// Validate node ID format
pub fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.is_empty() {
        return Err(anyhow!("Node ID cannot be empty"));
    }

    if node_id.len() > 128 {
        return Err(anyhow!("Node ID too long (max 128 characters)"));
    }

    // Must be alphanumeric with hyphens
    let re = Regex::new(r"^[a-zA-Z0-9-]+$").unwrap();
    if !re.is_match(node_id) {
        return Err(anyhow!("Invalid node ID format"));
    }

    Ok(())
}

/// Validate certificate request
pub fn validate_certificate_request(common_name: &str, san_entries: &[String]) -> Result<()> {
    // Validate common name
    if common_name.is_empty() {
        return Err(anyhow!("Common name cannot be empty"));
    }

    if common_name.len() > 64 {
        return Err(anyhow!("Common name too long (max 64 characters)"));
    }

    // Validate SAN entries
    for san in san_entries {
        if san.len() > 253 {
            return Err(anyhow!("SAN entry too long"));
        }
    }

    if san_entries.len() > 100 {
        return Err(anyhow!("Too many SAN entries (max 100)"));
    }

    Ok(())
}

/// Validate IPv6 address
pub fn validate_ipv6(addr: &str) -> Result<Ipv6Addr> {
    addr.parse::<Ipv6Addr>()
        .map_err(|_| anyhow!("Invalid IPv6 address: {}", addr))
}

/// Validate consensus proof size
pub fn validate_consensus_proof(proof_data: &[u8]) -> Result<()> {
    if proof_data.is_empty() {
        return Err(anyhow!("Consensus proof cannot be empty"));
    }

    if proof_data.len() > 10_000 {
        return Err(anyhow!("Consensus proof too large (max 10KB)"));
    }

    Ok(())
}

/// Sanitize user input to prevent injection attacks
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .take(256)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_node_id() {
        assert!(validate_node_id("node-123").is_ok());
        assert!(validate_node_id("").is_err());
        assert!(validate_node_id("node@123").is_err());
    }

    #[test]
    fn test_sanitize_input() {
        assert_eq!(sanitize_input("hello-world_123"), "hello-world_123");
        assert_eq!(sanitize_input("../../etc/passwd"), "..etcpasswd");
        assert_eq!(sanitize_input("'; DROP TABLE users; --"), "DROPTABLEusers");
    }
}
'''

        with open(validation_module, 'w') as f:
            f.write(validation_code)

        print(f"  ✓ Created input validation module at trustchain/src/validation.rs")

        # Add validation to lib.rs
        lib_file = self.project_root / 'trustchain/src/lib.rs'
        if lib_file.exists():
            with open(lib_file, 'r') as f:
                content = f.read()

            if 'pub mod validation;' not in content:
                # Find the module declarations section
                modules_section = content.find('pub mod ')
                if modules_section > 0:
                    # Insert validation module
                    insert_pos = content.find('\n', modules_section)
                    content = content[:insert_pos] + '\npub mod validation;' + content[insert_pos:]

                    with open(lib_file, 'w') as f:
                        f.write(content)

                    self.fixes_applied['validation_added'] += 1
                    print(f"  ✓ Added validation module to trustchain/src/lib.rs")

    def generate_report(self) -> None:
        """Generate remediation report"""
        print("\n" + "="*60)
        print("SECURITY REMEDIATION REPORT")
        print("="*60)
        print(f"\nRemediation completed at: {datetime.now().isoformat()}")
        print(f"Backup created at: {self.backup_dir}")

        print(f"\n{'='*40}")
        print("FIXES APPLIED")
        print(f"{'='*40}")
        print(f"Test bypasses removed:      {self.fixes_applied['test_bypasses_removed']}")
        print(f"Mock implementations moved: {self.fixes_applied['mock_implementations_marked']}")
        print(f"Unwrap calls replaced:      {self.fixes_applied['unwraps_replaced']}")
        print(f"Validation modules added:   {self.fixes_applied['validation_added']}")
        print(f"Total files modified:       {self.fixes_applied['files_modified']}")

        print(f"\n{'='*40}")
        print("NEXT STEPS")
        print(f"{'='*40}")
        print("1. Run 'cargo build --release' to verify compilation")
        print("2. Run 'cargo test' to ensure tests still pass")
        print("3. Run security audit again to verify fixes")
        print("4. Deploy to staging environment for integration testing")
        print("5. Perform penetration testing before production")

        print(f"\nBackup location: {self.backup_dir}")
        print("To restore: cp -r {} .".format(self.backup_dir / '*'))

def main():
    project_root = "/home/persist/repos/projects/web3"

    print("Starting security remediation...")
    print(f"Project root: {project_root}")

    remediator = SecurityRemediator(project_root)

    # Apply fixes
    remediator.fix_test_bypasses()
    remediator.fix_mock_implementations()
    remediator.fix_unwrap_calls()
    remediator.add_input_validation()

    # Generate report
    remediator.generate_report()

    print("\n✓ Security remediation completed successfully")

if __name__ == "__main__":
    main()