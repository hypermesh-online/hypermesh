# Web3 Ecosystem Scripts

## Directory Structure

```
scripts/
├── build/         # Build and compilation scripts
├── deployment/    # Deployment and infrastructure scripts
├── testing/       # Test execution and validation scripts
├── tools/         # Utility and helper scripts
└── archive/       # Legacy and deprecated scripts
```

## Key Scripts

### Build Scripts
- `build-ui.sh` - Build the UI frontend
- `build-all.sh` - Build all components

### Deployment Scripts
- `setup-local-dns.sh` - Configure local DNS for development
- `deploy-cluster.sh` - Deploy HyperMesh cluster
- `manage-cluster.sh` - Cluster management utilities

### Testing Scripts
- `run_dns_ct_tests.sh` - DNS/CT test suite
- `run_sprint2_tests.sh` - Sprint 2 validation tests

### Tools
- `sync-repos.sh` - Synchronize submodule repositories
- Various utility scripts for development

## Usage

All scripts should be run from the repository root:

```bash
./scripts/deployment/setup-local-dns.sh
./scripts/build/build-ui.sh
./scripts/testing/run_dns_ct_tests.sh
```

## Conventions

- Shell scripts use `.sh` extension
- Python scripts use `.py` extension
- Scripts are executable (`chmod +x`)
- Include help text with `-h` or `--help` flag
- Use consistent error handling and logging