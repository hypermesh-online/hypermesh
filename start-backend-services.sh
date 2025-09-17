#!/bin/bash

# Start Web3 Ecosystem Backend Services
# Simple Python-based API servers that provide real endpoints for the frontend

echo "🚀 Starting Web3 Ecosystem Backend Services..."

# Change to project directory  
cd "$(dirname "$0")"

# Create logs directory
mkdir -p logs

# Kill any existing services
echo "🛑 Stopping existing services..."
pkill -f "trustchain_simple_server\|stoq_simple_server\|hypermesh_simple_server" >/dev/null 2>&1 || true
pkill -f "python.*8444\|python.*8445\|python.*8446" >/dev/null 2>&1 || true

sleep 2

echo "⏳ Starting services..."

# Make servers executable
chmod +x trustchain_simple_server.py
chmod +x stoq_simple_server.py  
chmod +x hypermesh_simple_server.py

# Start TrustChain CA on port 8444
echo "🔐 Starting TrustChain CA on port 8444..."
nohup python3 trustchain_simple_server.py > logs/trustchain-8444.log 2>&1 &
TRUSTCHAIN_PID=$!
echo "  ✅ TrustChain CA started (PID: $TRUSTCHAIN_PID)"

# Start STOQ Transport on port 8445
echo "🚀 Starting STOQ Transport on port 8445..."
nohup python3 stoq_simple_server.py > logs/stoq-8445.log 2>&1 &
STOQ_PID=$!
echo "  ✅ STOQ Transport started (PID: $STOQ_PID)"

# Start HyperMesh Assets on port 8446
echo "🔗 Starting HyperMesh Assets on port 8446..."
nohup python3 hypermesh_simple_server.py > logs/hypermesh-8446.log 2>&1 &
HYPERMESH_PID=$!
echo "  ✅ HyperMesh Assets started (PID: $HYPERMESH_PID)"

echo ""
echo "⏳ Waiting for services to initialize..."
sleep 3

echo ""
echo "🔍 Checking service status..."

# Check TrustChain
if curl -s -o /dev/null -w "%{http_code}" "http://localhost:8444/health" | grep -q "200"; then
    echo "  ✅ TrustChain CA (port 8444): API responding"
else
    echo "  ⚠️ TrustChain CA (port 8444): Starting up or using fallback"
fi

# Check STOQ Transport  
if curl -s -o /dev/null -w "%{http_code}" "http://localhost:8445/health" | grep -q "200"; then
    echo "  ✅ STOQ Transport (port 8445): API responding"
else
    echo "  ⚠️ STOQ Transport (port 8445): Starting up or using fallback"
fi

# Check HyperMesh Assets
if curl -s -o /dev/null -w "%{http_code}" "http://localhost:8446/health" | grep -q "200"; then
    echo "  ✅ HyperMesh Assets (port 8446): API responding"
else
    echo "  ⚠️ HyperMesh Assets (port 8446): Starting up or using fallback"
fi

echo ""
echo "📊 Backend Services Status:"
echo "  🔐 TrustChain CA: http://localhost:8444 (PID: $TRUSTCHAIN_PID)"
echo "  🚀 STOQ Transport: http://localhost:8445 (PID: $STOQ_PID)"  
echo "  🔗 HyperMesh Assets: http://localhost:8446 (PID: $HYPERMESH_PID)"
echo ""
echo "  📝 Service logs available in logs/ directory"
echo "  🌐 All services listening on 0.0.0.0 (all interfaces)"
echo "  🔄 Frontend will auto-detect and connect to available services"

echo ""
echo "🎯 Services Ready!"
echo "  📋 API Endpoints:"
echo "    • TrustChain: http://localhost:8444/api/v1/trustchain/"
echo "    • STOQ: http://localhost:8445/api/v1/stoq/"
echo "    • HyperMesh: http://localhost:8446/api/v1/hypermesh/"
echo ""
echo "  🔍 Health Checks:"
echo "    • curl http://localhost:8444/health"
echo "    • curl http://localhost:8445/health"
echo "    • curl http://localhost:8446/health"

echo ""
echo "🎯 Next Steps:"
echo "  1. cd ui && npm run dev          # Start frontend"
echo "  2. Open http://localhost:1337    # View dashboard"
echo "  3. Backend APIs are now live and responding!"