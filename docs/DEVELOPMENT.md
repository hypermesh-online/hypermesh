# Web3 Ecosystem Development Setup

## Prerequisites

### Development Environment
- **OS**: Linux, macOS, or WSL2 on Windows
- **CPU**: 4+ cores recommended
- **RAM**: 16GB minimum
- **Storage**: 100GB available space

### Required Tools
```bash
# Check versions
rustc --version    # 1.75+ required
node --version     # v20+ required
docker --version   # 24.0+ required
git --version      # 2.40+ required

# Install if missing
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://get.docker.com | sh
```

## Repository Setup

### 1. Clone Main Repository
```bash
git clone https://github.com/hypermesh-online/web3-ecosystem.git
cd web3-ecosystem

# Initialize submodules if any
git submodule update --init --recursive
```

### 2. Component Repositories
```bash
# Clone individual components (if working separately)
git clone https://github.com/hypermesh-online/trustchain.git
git clone https://github.com/hypermesh-online/stoq.git
git clone https://github.com/hypermesh-online/hypermesh.git
git clone https://github.com/hypermesh-online/caesar.git
git clone https://github.com/hypermesh-online/catalog.git
git clone https://github.com/hypermesh-online/ngauge.git
```

## Build Setup

### Rust Components
```bash
# Install Rust toolchain
rustup default stable
rustup component add rustfmt clippy

# Build all Rust components
make build-rust

# Or build individually
cd trustchain && cargo build
cd stoq && cargo build
cd hypermesh && cargo build
```

### Node.js Components
```bash
# Install dependencies
npm install -g pnpm
pnpm install

# Build frontend
cd ui/frontend
pnpm build

# Build Caesar contracts
cd caesar
pnpm hardhat compile
```

## Local Development Environment

### Docker Compose Setup
```bash
# Copy environment template
cp .env.example .env

# Edit environment variables
vim .env
# Set:
# ENVIRONMENT=development
# TRUSTCHAIN_MODE=self-signed
# STOQ_BIND=[::1]:443
# HYPERMESH_NODES=1

# Start development stack
docker-compose -f docker-compose.dev.yml up
```

### Manual Component Start

#### TrustChain (Port 8443)
```bash
cd trustchain
cargo run -- serve \
  --dev-mode \
  --bind "[::1]:8443" \
  --self-signed
```

#### STOQ (Port 8444)
```bash
cd stoq
cargo run --bin stoq-server -- \
  --bind "[::1]:8444" \
  --trustchain "https://[::1]:8443" \
  --insecure
```

#### HyperMesh (Port 8545)
```bash
cd hypermesh
cargo run -- \
  --dev \
  --bind "[::1]:8545" \
  --trustchain "https://[::1]:8443" \
  --stoq "stoq://[::1]:8444"
```

#### Caesar (Port 8080)
```bash
cd caesar

# Deploy local contracts
npx hardhat node &
npx hardhat deploy --network localhost

# Start API server
npm run dev
```

## Testing

### Unit Tests
```bash
# Run all unit tests
make test

# Component-specific tests
cd trustchain && cargo test
cd stoq && cargo test
cd hypermesh && cargo test
cd caesar && npm test
```

### Integration Tests
```bash
# Start test environment
docker-compose -f docker-compose.test.yml up -d

# Run integration suite
npm run test:integration

# Cleanup
docker-compose -f docker-compose.test.yml down
```

### E2E Tests
```bash
# Start full stack
./scripts/start-e2e-env.sh

# Run E2E tests
npm run test:e2e

# View results
open test-results/e2e-report.html
```

## Development Tools

### IDE Setup

#### VS Code
```json
// .vscode/extensions.json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb",
    "dbaeumer.vscode-eslint",
    "esbenp.prettier-vscode"
  ]
}
```

#### IntelliJ IDEA / RustRover
- Install Rust plugin
- Configure Cargo projects
- Set up Docker integration

### Debugging

#### Rust Components
```bash
# Debug build with symbols
cargo build --debug

# Run with logging
RUST_LOG=debug cargo run

# Use debugger
rust-gdb target/debug/hypermesh
```

#### Node.js Components
```bash
# Debug mode
node --inspect npm run dev

# Chrome DevTools
chrome://inspect

# VS Code debugger
# F5 with launch.json configured
```

### Performance Profiling

#### Flamegraphs
```bash
# Install flamegraph
cargo install flamegraph

# Profile application
cargo flamegraph --bin hypermesh

# View results
open flamegraph.svg
```

#### Benchmarks
```bash
# Run benchmarks
cargo bench

# Criterion reports
open target/criterion/report/index.html
```

## Common Development Tasks

### Adding New Features
```bash
# Create feature branch
git checkout -b feature/my-feature

# Make changes
vim src/my_feature.rs

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Commit
git add -A
git commit -m "feat: add my feature"
```

### Updating Dependencies
```bash
# Rust dependencies
cargo update
cargo outdated

# Node dependencies
pnpm update
pnpm outdated
```

### Database Migrations
```bash
# Create migration
diesel migration generate add_new_table

# Edit migration
vim migrations/*/up.sql

# Run migrations
diesel migration run

# Rollback
diesel migration revert
```

## Troubleshooting

### Build Issues

#### Rust Compilation Errors
```bash
# Clean build
cargo clean
cargo build

# Update toolchain
rustup update
```

#### Node Module Issues
```bash
# Clear cache
pnpm store prune
rm -rf node_modules
pnpm install
```

### Runtime Issues

#### Port Already in Use
```bash
# Find process using port
lsof -i :8443

# Kill process
kill -9 <PID>
```

#### IPv6 Issues
```bash
# Enable IPv6 (Linux)
sudo sysctl -w net.ipv6.conf.all.disable_ipv6=0

# Test IPv6
ping6 ::1
```

#### Certificate Issues
```bash
# Regenerate certificates
cd scripts
./generate-certs.sh --dev
```

## Development Best Practices

### Code Style
- Use `rustfmt` for Rust code
- Use `prettier` for JavaScript/TypeScript
- Follow project conventions in `.editorconfig`

### Commit Messages
```
type(scope): description

- feat: New feature
- fix: Bug fix
- docs: Documentation
- refactor: Code refactoring
- test: Test updates
- chore: Maintenance
```

### Testing
- Write tests for new features
- Maintain >80% code coverage
- Run tests before committing

### Documentation
- Update docs with code changes
- Include examples in doc comments
- Keep README files current

## Resources

### Documentation
- [Architecture Overview](./ARCHITECTURE.md)
- [API Reference](./guides/api-reference.md)
- [Component Guides](./components/)

### Community
- Discord: https://discord.gg/hypermesh
- GitHub Discussions: https://github.com/hypermesh-online/ecosystem/discussions

### Tools
- [Rust Book](https://doc.rust-lang.org/book/)
- [QUIC Specification](https://www.rfc-editor.org/rfc/rfc9000.html)
- [Ethereum Development](https://ethereum.org/developers)