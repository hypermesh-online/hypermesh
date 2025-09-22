# Quick Start Guide

Get Hypermesh Nexus running in under 5 minutes!

## 🚀 Installation

### Prerequisites
```bash
# Ensure you have Rust 1.75+
rustup update stable

# Install system dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config clang llvm

# For eBPF support (optional but recommended)
sudo apt-get install -y linux-headers-$(uname -r) libelf-dev
```

### Get the Code
```bash
git clone https://github.com/your-org/hypermesh.git
cd hypermesh
```

## ⚡ 2-Minute Demo

### 1. Run the Test Suite
```bash
cd core/tests
cargo run --bin nexus-test -- all --detailed-report
```

**Expected Output:**
```
🚀 Nexus Testing Framework v1.0.0
🧪 Running unit tests
  ✅ Runtime component tests passed
  ✅ Transport layer tests passed  
  ✅ Consensus engine tests passed
  ✅ Networking tests passed
  ✅ eBPF integration tests passed
  ✅ Storage layer tests passed
✅ Tests completed successfully
📄 Report saved to: nexus-test-report.md
```

### 2. Deploy Local Cluster
```bash
cd ../deploy
./deploy.sh deploy --type local --size 3 --env staging --verbose
```

**Expected Output:**
```
🚀 Starting Nexus deployment...
Configuration:
  Type: local
  Size: 3 nodes  
  Environment: staging
✅ System requirements validated
✅ Nexus components built successfully
✅ Pre-deployment tests passed
✅ Configuration generated for 3 nodes
✅ 3-node local cluster deployed successfully
```

### 3. Verify Deployment
```bash
./deploy.sh status
```

## 🎯 What Just Happened?

You just:
1. **Validated** all Nexus core components work correctly
2. **Deployed** a 3-node Byzantine fault-tolerant cluster  
3. **Verified** the cluster is running and healthy

The cluster includes:
- **QUIC transport** with TLS encryption between nodes
- **Byzantine consensus** for distributed state management
- **eBPF networking** for high-performance packet processing
- **Professional monitoring** with real-time metrics

## 🔧 Next Steps

### Explore Individual Components

#### Test Transport Layer
```bash
cd core/tests
cargo run --bin nexus-test -- unit --component transport
```

#### Test Consensus Engine  
```bash
cargo run --bin nexus-test -- unit --component consensus
```

#### Run Performance Benchmarks
```bash
cargo run --bin nexus-test -- unit --component runtime
cargo run --bin nexus-test -- unit --component networking
```

### Deploy to Different Environments

#### Staging Environment (5 nodes)
```bash
./deploy.sh deploy --type local --size 5 --env staging
```

#### Production Environment (7 nodes)
```bash  
./deploy.sh deploy --type systemd --size 7 --env prod
```

#### Docker Environment
```bash
./deploy.sh deploy --type docker --size 3 --env dev
```

### Monitor Performance
```bash
# Real-time metrics for 60 seconds
cargo run --bin nexus-test -- metrics --duration 60 --real-time

# Run deployment tests
cargo run --bin nexus-test -- deploy --nodes 5

# Run staging tests
cargo run --bin nexus-test -- staging --size 7 --deployment local
```

## 📊 Understanding the Output

### Test Results
- **Unit Tests**: Individual component validation
- **Deployment Tests**: Multi-node cluster functionality  
- **Metrics Tests**: Performance and monitoring
- **Staging Tests**: Production-like environment validation

### Deployment Status
- **Local**: Processes running on local machine
- **Docker**: Containerized deployment (if Docker available)  
- **Systemd**: System service deployment
- **Status**: Health check of all nodes

## 🔍 Troubleshooting

### Build Errors
```bash
# Clean and rebuild
cd core
cargo clean
cargo build --release
```

### Port Conflicts
```bash
# Check for port usage
netstat -tulpn | grep :8080
netstat -tulpn | grep :8081

# Deploy with different ports
./deploy.sh deploy --type local --size 3 --config custom-ports.toml
```

### Permission Issues
```bash
# For eBPF features (requires root)
sudo cargo run --bin nexus-test -- unit --component ebpf

# For systemd deployment
sudo ./deploy.sh deploy --type systemd --size 3
```

## 📚 What's Next?

1. **[Architecture Guide](architecture.md)** - Understand how it works
2. **[Deployment Guide](deployment.md)** - Production deployment strategies  
3. **[API Reference](api/)** - Integrate with your applications
4. **[Examples](../examples/)** - Real-world use cases
5. **[Developer Guide](developer-guide.md)** - Contribute to the project

## 🎮 Advanced Usage

### Custom Configuration
```bash
# Generate sample configuration
./deploy.sh deploy --dry-run --verbose > my-config.yaml

# Deploy with custom config
./deploy.sh deploy --config my-config.yaml
```

### CI/CD Integration
```bash
# The project includes GitHub Actions workflow
cat ../.github/workflows/nexus-tests.yml

# Run same tests as CI
cargo run --bin nexus-test -- all --detailed-report
```

### Cleanup
```bash
# Stop and clean deployment
./deploy.sh clean

# Clean build artifacts  
cargo clean
```

---

**🎉 Congratulations!** You now have a working Hypermesh Nexus cluster. Ready to replace Kubernetes? 

Check out the [Architecture Guide](architecture.md) to understand what makes Nexus different!