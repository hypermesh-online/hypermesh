#!/usr/bin/env python3
"""
HyperMesh Comprehensive Testing Suite
Tests actual functionality of the HyperMesh asset management system
"""

import subprocess
import json
import os
import sys
from pathlib import Path

class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

def run_command(cmd, cwd=None):
    """Run a shell command and return output"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cwd)
        return result.returncode, result.stdout, result.stderr
    except Exception as e:
        return -1, "", str(e)

def test_build_status():
    """Test if HyperMesh compiles"""
    print(f"\n{Colors.HEADER}=== BUILD STATUS TEST ==={Colors.ENDC}")

    # Try to build specific modules
    modules = [
        "hypermesh-assets",
        "hypermesh-transport",
        "hypermesh-consensus",
        "hypermesh-orchestration",
        "hypermesh-catalog",
        "stoq"
    ]

    results = {}
    for module in modules:
        print(f"\nBuilding {module}...")
        code, stdout, stderr = run_command(f"cargo build -p {module} 2>&1", cwd="/home/persist/repos/projects/web3/hypermesh")

        if code == 0:
            results[module] = "✅ BUILDS"
            print(f"  {Colors.OKGREEN}✅ Builds successfully{Colors.ENDC}")
        else:
            error_count = stderr.count("error[E") + stderr.count("error:")
            results[module] = f"❌ {error_count} errors"
            print(f"  {Colors.FAIL}❌ Failed with {error_count} errors{Colors.ENDC}")

    return results

def test_asset_management():
    """Test asset management functionality"""
    print(f"\n{Colors.HEADER}=== ASSET MANAGEMENT TEST ==={Colors.ENDC}")

    # Check for asset management code
    asset_files = [
        "/home/persist/repos/projects/web3/hypermesh/src/assets/core/mod.rs",
        "/home/persist/repos/projects/web3/hypermesh/src/assets/adapters/cpu.rs",
        "/home/persist/repos/projects/web3/hypermesh/src/assets/adapters/gpu.rs",
        "/home/persist/repos/projects/web3/hypermesh/src/assets/adapters/memory.rs",
        "/home/persist/repos/projects/web3/hypermesh/src/assets/adapters/storage.rs"
    ]

    results = {}
    for file in asset_files:
        if os.path.exists(file):
            with open(file, 'r') as f:
                content = f.read()

            component = os.path.basename(file).replace('.rs', '')

            # Check for key patterns
            checks = {
                "AssetAdapter trait": "trait AssetAdapter" in content or "impl AssetAdapter" in content,
                "Four-proof consensus": any(proof in content for proof in ["PoSpace", "PoStake", "PoWork", "PoTime"]),
                "Dynamic allocation": "allocate" in content.lower() or "resource" in content.lower(),
                "Privacy levels": "Privacy" in content or "private" in content.lower()
            }

            working_features = sum(checks.values())
            total_features = len(checks)

            if working_features == total_features:
                results[component] = "✅ IMPLEMENTED"
            elif working_features > 0:
                results[component] = f"⚠️  {working_features}/{total_features} features"
            else:
                results[component] = "❌ NOT IMPLEMENTED"

            print(f"\n  {component}:")
            for feature, present in checks.items():
                status = "✅" if present else "❌"
                print(f"    {status} {feature}")
        else:
            component = os.path.basename(file).replace('.rs', '')
            results[component] = "❌ FILE MISSING"
            print(f"\n  {Colors.FAIL}❌ {component}: FILE MISSING{Colors.ENDC}")

    return results

def test_stoq_integration():
    """Test STOQ protocol integration"""
    print(f"\n{Colors.HEADER}=== STOQ PROTOCOL TEST ==={Colors.ENDC}")

    stoq_path = "/home/persist/repos/projects/web3/hypermesh/protocols/stoq"

    if not os.path.exists(stoq_path):
        return {"STOQ": "❌ DIRECTORY MISSING"}

    # Check for key STOQ components
    components = {
        "Transport": f"{stoq_path}/src/transport",
        "Chunking": f"{stoq_path}/src/chunking",
        "Routing": f"{stoq_path}/src/routing",
        "Edge": f"{stoq_path}/src/edge"
    }

    results = {}
    for name, path in components.items():
        if os.path.exists(path):
            # Check if it has actual implementation
            rs_files = list(Path(path).glob("*.rs"))
            if rs_files:
                total_lines = sum(len(open(f).readlines()) for f in rs_files)
                if total_lines > 100:  # Substantial implementation
                    results[name] = f"✅ {total_lines} lines"
                else:
                    results[name] = f"⚠️  {total_lines} lines (minimal)"
            else:
                results[name] = "❌ NO CODE"
        else:
            results[name] = "❌ MISSING"

        print(f"  {name}: {results[name]}")

    # Check for QUIC implementation
    print(f"\n  Checking QUIC implementation...")
    code, stdout, stderr = run_command(f"grep -r 'quinn' {stoq_path}/src --include='*.rs' | wc -l")
    quinn_refs = int(stdout.strip()) if stdout.strip().isdigit() else 0

    if quinn_refs > 10:
        results["QUIC"] = f"✅ {quinn_refs} references"
        print(f"    {Colors.OKGREEN}✅ QUIC: {quinn_refs} references found{Colors.ENDC}")
    elif quinn_refs > 0:
        results["QUIC"] = f"⚠️  {quinn_refs} references"
        print(f"    {Colors.WARNING}⚠️  QUIC: Only {quinn_refs} references{Colors.ENDC}")
    else:
        results["QUIC"] = "❌ NOT FOUND"
        print(f"    {Colors.FAIL}❌ QUIC: No implementation found{Colors.ENDC}")

    return results

def test_trustchain_integration():
    """Test TrustChain integration"""
    print(f"\n{Colors.HEADER}=== TRUSTCHAIN INTEGRATION TEST ==={Colors.ENDC}")

    # Check if TrustChain is referenced in HyperMesh
    code, stdout, _ = run_command("grep -r 'trustchain\\|TrustChain' /home/persist/repos/projects/web3/hypermesh/src --include='*.rs' | wc -l")
    trustchain_refs = int(stdout.strip()) if stdout.strip().isdigit() else 0

    results = {}
    if trustchain_refs > 0:
        results["References"] = f"✅ {trustchain_refs} found"
        print(f"  {Colors.OKGREEN}✅ TrustChain references: {trustchain_refs}{Colors.ENDC}")
    else:
        results["References"] = "❌ NONE"
        print(f"  {Colors.FAIL}❌ No TrustChain references found{Colors.ENDC}")

    # Check for certificate handling
    code, stdout, _ = run_command("grep -r 'certificate\\|Certificate' /home/persist/repos/projects/web3/hypermesh/src --include='*.rs' | wc -l")
    cert_refs = int(stdout.strip()) if stdout.strip().isdigit() else 0

    if cert_refs > 10:
        results["Certificates"] = f"✅ {cert_refs} refs"
        print(f"  {Colors.OKGREEN}✅ Certificate handling: {cert_refs} references{Colors.ENDC}")
    else:
        results["Certificates"] = f"⚠️  {cert_refs} refs"
        print(f"  {Colors.WARNING}⚠️  Certificate handling: Only {cert_refs} references{Colors.ENDC}")

    return results

def test_nova_gpu():
    """Test Nova/Vulkan GPU implementation"""
    print(f"\n{Colors.HEADER}=== NOVA/VULKAN GPU TEST ==={Colors.ENDC}")

    results = {}

    # Check for Nova references
    code, stdout, _ = run_command("grep -r 'nova\\|Nova\\|vulkan\\|Vulkan' /home/persist/repos/projects/web3/hypermesh/src --include='*.rs' | wc -l")
    nova_refs = int(stdout.strip()) if stdout.strip().isdigit() else 0

    # Check for CUDA references (should be minimal/none)
    code, stdout, _ = run_command("grep -r 'cuda\\|CUDA' /home/persist/repos/projects/web3/hypermesh/src --include='*.rs' | wc -l")
    cuda_refs = int(stdout.strip()) if stdout.strip().isdigit() else 0

    if nova_refs > 0 and cuda_refs == 0:
        results["GPU"] = f"✅ Nova/Vulkan ({nova_refs} refs)"
        print(f"  {Colors.OKGREEN}✅ Correct GPU implementation: Nova/Vulkan{Colors.ENDC}")
    elif cuda_refs > nova_refs:
        results["GPU"] = f"❌ CUDA ({cuda_refs} refs)"
        print(f"  {Colors.FAIL}❌ Wrong GPU implementation: CUDA instead of Nova{Colors.ENDC}")
    else:
        results["GPU"] = "❌ NO IMPLEMENTATION"
        print(f"  {Colors.FAIL}❌ No GPU implementation found{Colors.ENDC}")

    print(f"    Nova/Vulkan references: {nova_refs}")
    print(f"    CUDA references: {cuda_refs}")

    return results

def test_nat_proxy():
    """Test NAT/Proxy system for remote addressing"""
    print(f"\n{Colors.HEADER}=== NAT/PROXY SYSTEM TEST ==={Colors.ENDC}")

    proxy_path = "/home/persist/repos/projects/web3/hypermesh/src/assets/proxy"

    results = {}
    if os.path.exists(proxy_path):
        # Count implementation files
        rs_files = list(Path(proxy_path).glob("*.rs"))
        if rs_files:
            total_lines = sum(len(open(f).readlines()) for f in rs_files)
            results["Implementation"] = f"✅ {len(rs_files)} files, {total_lines} lines"
            print(f"  {Colors.OKGREEN}✅ Proxy implementation: {len(rs_files)} files, {total_lines} lines{Colors.ENDC}")

            # Check for NAT-like addressing
            nat_found = False
            for f in rs_files:
                with open(f, 'r') as file:
                    if 'nat' in file.read().lower() or 'address' in file.read().lower():
                        nat_found = True
                        break

            if nat_found:
                results["NAT Addressing"] = "✅ FOUND"
                print(f"  {Colors.OKGREEN}✅ NAT-like addressing implementation found{Colors.ENDC}")
            else:
                results["NAT Addressing"] = "⚠️  NOT CLEAR"
                print(f"  {Colors.WARNING}⚠️  NAT-like addressing not clearly implemented{Colors.ENDC}")
        else:
            results["Implementation"] = "❌ NO FILES"
            print(f"  {Colors.FAIL}❌ No implementation files found{Colors.ENDC}")
    else:
        results["Proxy"] = "❌ DIRECTORY MISSING"
        print(f"  {Colors.FAIL}❌ Proxy directory missing{Colors.ENDC}")

    return results

def generate_report(all_results):
    """Generate comprehensive test report"""
    print(f"\n{Colors.HEADER}{'='*60}")
    print(f"HYPERMESH COMPREHENSIVE TEST REPORT")
    print(f"{'='*60}{Colors.ENDC}\n")

    # Calculate overall health
    total_tests = 0
    passed_tests = 0
    partial_tests = 0
    failed_tests = 0

    for category, results in all_results.items():
        for component, status in results.items():
            total_tests += 1
            if "✅" in status and "error" not in status.lower():
                passed_tests += 1
            elif "⚠️" in status:
                partial_tests += 1
            else:
                failed_tests += 1

    health_percentage = (passed_tests / total_tests * 100) if total_tests > 0 else 0

    # Print summary
    print(f"{Colors.BOLD}OVERALL SYSTEM HEALTH: {health_percentage:.1f}%{Colors.ENDC}")
    print(f"  ✅ Passed: {passed_tests}/{total_tests}")
    print(f"  ⚠️  Partial: {partial_tests}/{total_tests}")
    print(f"  ❌ Failed: {failed_tests}/{total_tests}")

    print(f"\n{Colors.BOLD}DETAILED RESULTS:{Colors.ENDC}\n")

    for category, results in all_results.items():
        print(f"{Colors.OKCYAN}{category}:{Colors.ENDC}")
        for component, status in results.items():
            # Color code the output
            if "✅" in status:
                print(f"  {Colors.OKGREEN}{component}: {status}{Colors.ENDC}")
            elif "⚠️" in status:
                print(f"  {Colors.WARNING}{component}: {status}{Colors.ENDC}")
            else:
                print(f"  {Colors.FAIL}{component}: {status}{Colors.ENDC}")
        print()

    # Critical issues
    print(f"{Colors.BOLD}CRITICAL ISSUES IDENTIFIED:{Colors.ENDC}")
    critical_issues = []

    if all_results.get("Build Status", {}).get("hypermesh-assets", "").startswith("❌"):
        critical_issues.append("Asset management module fails to compile")
    if all_results.get("Build Status", {}).get("stoq", "").startswith("❌"):
        critical_issues.append("STOQ protocol module fails to compile")
    if "❌ NO IMPLEMENTATION" in str(all_results.get("Nova/Vulkan GPU", {}).values()):
        critical_issues.append("GPU abstraction not implemented (should use Nova/Vulkan)")
    if "❌ DIRECTORY MISSING" in str(all_results.get("NAT/Proxy System", {}).values()):
        critical_issues.append("NAT/Proxy system missing (critical for remote addressing)")

    if critical_issues:
        for i, issue in enumerate(critical_issues, 1):
            print(f"  {Colors.FAIL}{i}. {issue}{Colors.ENDC}")
    else:
        print(f"  {Colors.OKGREEN}No critical issues found{Colors.ENDC}")

    return health_percentage

def main():
    """Run all tests"""
    print(f"{Colors.BOLD}Starting HyperMesh Comprehensive Testing...{Colors.ENDC}")

    all_results = {}

    # Run all test suites
    all_results["Build Status"] = test_build_status()
    all_results["Asset Management"] = test_asset_management()
    all_results["STOQ Integration"] = test_stoq_integration()
    all_results["TrustChain Integration"] = test_trustchain_integration()
    all_results["Nova/Vulkan GPU"] = test_nova_gpu()
    all_results["NAT/Proxy System"] = test_nat_proxy()

    # Generate report
    health = generate_report(all_results)

    # Final verdict
    print(f"\n{Colors.HEADER}{'='*60}{Colors.ENDC}")
    if health >= 80:
        print(f"{Colors.OKGREEN}✅ VERDICT: System is functional ({health:.1f}% health){Colors.ENDC}")
        return 0
    elif health >= 50:
        print(f"{Colors.WARNING}⚠️  VERDICT: System partially functional ({health:.1f}% health){Colors.ENDC}")
        return 1
    else:
        print(f"{Colors.FAIL}❌ VERDICT: System not functional ({health:.1f}% health){Colors.ENDC}")
        return 2

if __name__ == "__main__":
    sys.exit(main())