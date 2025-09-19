# Makefile for Web3 Ecosystem Development
# Provides convenient commands for local development and testing

.PHONY: help local-setup dns-setup build start stop restart status test clean

# Default target
help:
	@echo "Web3 Ecosystem Development Commands"
	@echo "==================================="
	@echo ""
	@echo "Setup Commands:"
	@echo "  local-setup    - Complete local development setup (DNS + certificates + build + start)"
	@echo "  dns-setup      - Configure local DNS routing only"
	@echo "  build          - Build the HyperMesh server"
	@echo ""
	@echo "Server Commands:"
	@echo "  start          - Start the HyperMesh server"
	@echo "  stop           - Stop the HyperMesh server"
	@echo "  restart        - Restart the HyperMesh server"
	@echo "  status         - Show server status"
	@echo ""
	@echo "Testing Commands:"
	@echo "  test           - Test DNS resolution and server connectivity"
	@echo "  test-dns       - Test DNS resolution only"
	@echo "  test-server    - Test server connectivity only"
	@echo ""
	@echo "Maintenance Commands:"
	@echo "  clean          - Remove DNS entries and certificates"
	@echo "  logs           - Show server logs"
	@echo "  logs-follow    - Follow server logs in real-time"
	@echo ""
	@echo "Docker Commands:"
	@echo "  docker-up      - Start containerized DNS and server"
	@echo "  docker-down    - Stop containerized services"
	@echo ""
	@echo "Required Domains:"
	@echo "  https://hypermesh.online:8443        - Main dashboard"
	@echo "  https://trust.hypermesh.online:8443  - TrustChain authority"
	@echo "  https://caesar.hypermesh.online:8443 - Caesar economics"
	@echo "  https://catalog.hypermesh.online:8443 - Catalog VM system"
	@echo "  https://stoq.hypermesh.online:8443   - STOQ transport"
	@echo "  https://ngauge.hypermesh.online:8443 - NGauge platform"

# Complete local development setup
local-setup: dns-setup build start test
	@echo ""
	@echo "🎉 Local development environment ready!"
	@echo ""
	@echo "Frontend: https://hypermesh.online:8443"
	@echo "Logs:     make logs-follow"
	@echo "Status:   make status"

# Configure local DNS routing
dns-setup:
	@echo "🌐 Setting up local DNS routing..."
	@sudo ./infrastructure/dns/local-dns-setup.sh setup

# Build the HyperMesh server
build:
	@echo "🔨 Building HyperMesh server..."
	@./deploy-hypermesh.sh build

# Start the server
start:
	@echo "🚀 Starting HyperMesh server..."
	@./deploy-hypermesh.sh start

# Stop the server
stop:
	@echo "🛑 Stopping HyperMesh server..."
	@./deploy-hypermesh.sh stop

# Restart the server
restart:
	@echo "🔄 Restarting HyperMesh server..."
	@./deploy-hypermesh.sh restart

# Show server status
status:
	@./deploy-hypermesh.sh status

# Test complete setup
test: test-dns test-server
	@echo ""
	@echo "✅ All tests completed"

# Test DNS resolution
test-dns:
	@echo "🔍 Testing DNS resolution..."
	@./infrastructure/dns/local-dns-setup.sh test

# Test server connectivity
test-server:
	@echo "🌐 Testing server connectivity..."
	@echo "Testing HTTPS connections..."
	@curl -k -s --connect-timeout 5 https://hypermesh.online:8443 > /dev/null && echo "✅ hypermesh.online" || echo "❌ hypermesh.online"
	@curl -k -s --connect-timeout 5 https://trust.hypermesh.online:8443 > /dev/null && echo "✅ trust.hypermesh.online" || echo "❌ trust.hypermesh.online"
	@curl -k -s --connect-timeout 5 https://caesar.hypermesh.online:8443 > /dev/null && echo "✅ caesar.hypermesh.online" || echo "❌ caesar.hypermesh.online"
	@curl -k -s --connect-timeout 5 https://catalog.hypermesh.online:8443 > /dev/null && echo "✅ catalog.hypermesh.online" || echo "❌ catalog.hypermesh.online"
	@curl -k -s --connect-timeout 5 https://stoq.hypermesh.online:8443 > /dev/null && echo "✅ stoq.hypermesh.online" || echo "❌ stoq.hypermesh.online"
	@curl -k -s --connect-timeout 5 https://ngauge.hypermesh.online:8443 > /dev/null && echo "✅ ngauge.hypermesh.online" || echo "❌ ngauge.hypermesh.online"

# Clean DNS entries and certificates
clean:
	@echo "🧹 Cleaning up local DNS setup..."
	@sudo ./infrastructure/dns/local-dns-setup.sh remove
	@echo "Removing build artifacts..."
	@rm -rf target/ certificates/ logs/ *.pid 2>/dev/null || true

# Show server logs
logs:
	@if [ -f logs/server.log ]; then \
		tail -n 50 logs/server.log; \
	else \
		echo "No log file found. Is the server running?"; \
	fi

# Follow server logs in real-time
logs-follow:
	@if [ -f logs/server.log ]; then \
		tail -f logs/server.log; \
	else \
		echo "No log file found. Is the server running?"; \
	fi

# Docker-based setup
docker-up:
	@echo "🐳 Starting containerized services..."
	@cd infrastructure/dns && docker-compose -f docker-dns-setup.yml up -d
	@echo "Services started:"
	@echo "  DNS server:     http://localhost:5380"
	@echo "  Internet2 API:  https://hypermesh.online:8443"

docker-down:
	@echo "🐳 Stopping containerized services..."
	@cd infrastructure/dns && docker-compose -f docker-dns-setup.yml down

# Development helpers
dev-config:
	@echo "📝 Generating development configuration..."
	@./infrastructure/dns/local-dns-setup.sh config

certificates:
	@echo "🔐 Generating SSL certificates..."
	@./infrastructure/dns/local-dns-setup.sh cert

hosts:
	@echo "📝 Configuring hosts file..."
	@sudo ./infrastructure/dns/local-dns-setup.sh hosts

# Verification commands
verify-setup:
	@echo "🔍 Verifying local setup..."
	@echo "Checking DNS resolution..."
	@for domain in hypermesh.online trust.hypermesh.online caesar.hypermesh.online catalog.hypermesh.online stoq.hypermesh.online ngauge.hypermesh.online; do \
		if ping -c 1 -W 2 $$domain > /dev/null 2>&1; then \
			echo "✅ $$domain resolves"; \
		else \
			echo "❌ $$domain failed to resolve"; \
		fi; \
	done
	@echo ""
	@echo "Checking certificates..."
	@if [ -f certificates/hypermesh-ca.crt ]; then \
		echo "✅ CA certificate exists"; \
	else \
		echo "❌ CA certificate missing"; \
	fi
	@if [ -f certificates/hypermesh-server.crt ]; then \
		echo "✅ Server certificate exists"; \
	else \
		echo "❌ Server certificate missing"; \
	fi
	@echo ""
	@echo "Checking server..."
	@if netstat -tuln 2>/dev/null | grep -q ":8443 "; then \
		echo "✅ Server listening on port 8443"; \
	else \
		echo "❌ Server not listening on port 8443"; \
	fi

# Quick development cycle
dev: build restart test
	@echo "🔄 Development cycle complete"

# Production preparation
prep-prod:
	@echo "🏭 Preparing for production..."
	@echo "Building with production features..."
	@cargo build --release --features production
	@echo "Validating production configuration..."
	@if [ -f config/production.toml ]; then \
		echo "✅ Production config exists"; \
	else \
		echo "❌ Production config missing"; \
	fi
	@echo "⚠️  Remember to:"
	@echo "  - Use real domain names"
	@echo "  - Configure real SSL certificates"
	@echo "  - Enable full consensus validation"
	@echo "  - Set production security settings"

# Show environment info
info:
	@echo "Web3 Ecosystem Environment Information"
	@echo "====================================="
	@echo "Platform: $(shell uname -s)"
	@echo "Architecture: $(shell uname -m)"
	@echo "Working Directory: $(shell pwd)"
	@echo ""
	@echo "Dependencies:"
	@if command -v cargo > /dev/null; then \
		echo "  ✅ Rust/Cargo: $(shell cargo --version)"; \
	else \
		echo "  ❌ Rust/Cargo not found"; \
	fi
	@if command -v openssl > /dev/null; then \
		echo "  ✅ OpenSSL: $(shell openssl version)"; \
	else \
		echo "  ❌ OpenSSL not found"; \
	fi
	@if command -v curl > /dev/null; then \
		echo "  ✅ curl: $(shell curl --version | head -n1)"; \
	else \
		echo "  ❌ curl not found"; \
	fi
	@if command -v docker > /dev/null; then \
		echo "  ✅ Docker: $(shell docker --version)"; \
	else \
		echo "  ⚠️  Docker not found (optional)"; \
	fi
	@echo ""
	@echo "Network:"
	@if ping -c 1 -W 2 127.0.0.1 > /dev/null 2>&1; then \
		echo "  ✅ IPv4 localhost reachable"; \
	else \
		echo "  ❌ IPv4 localhost unreachable"; \
	fi
	@if ping6 -c 1 -W 2 ::1 > /dev/null 2>&1; then \
		echo "  ✅ IPv6 localhost reachable"; \
	else \
		echo "  ⚠️  IPv6 localhost unreachable"; \
	fi

# Install development dependencies
install-deps:
	@echo "📦 Installing development dependencies..."
	@echo "Checking system package manager..."
	@if command -v apt-get > /dev/null; then \
		echo "Using apt-get (Ubuntu/Debian)..."; \
		sudo apt-get update && sudo apt-get install -y openssl curl nettools-ping build-essential pkg-config libssl-dev cmake; \
	elif command -v yum > /dev/null; then \
		echo "Using yum (CentOS/RHEL)..."; \
		sudo yum install -y openssl curl iputils gcc gcc-c++ openssl-devel cmake; \
	elif command -v brew > /dev/null; then \
		echo "Using Homebrew (macOS)..."; \
		brew install openssl curl cmake; \
	else \
		echo "⚠️  Unknown package manager. Please install manually:"; \
		echo "  - OpenSSL"; \
		echo "  - curl"; \
		echo "  - build-essential (gcc, cmake, etc.)"; \
	fi
	@echo "✅ Dependencies installation complete"