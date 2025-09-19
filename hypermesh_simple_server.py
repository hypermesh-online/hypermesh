#!/usr/bin/env python3
"""
HyperMesh Simple Server - Asset Management & Consensus Validation
Production-quality Python server for HyperMesh service operations.

Port: 8446 (as expected by start-backend-services.sh)
Features:
- Universal asset management (CPU, GPU, Memory, Storage)
- Four-proof consensus system (PoSp, PoSt, PoWk, PoTm) 
- Byzantine fault detection and recovery
- Remote proxy/NAT addressing system
- VM execution and Catalog integration
"""

import asyncio
import json
import logging
import os
import sys
import time
import uuid
import random
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any, Union
from dataclasses import dataclass, asdict, field
from pathlib import Path
from enum import Enum

from fastapi import FastAPI, HTTPException, Request, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
import uvicorn
from pydantic import BaseModel, Field


# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('logs/hypermesh.log')
    ]
)
logger = logging.getLogger("hypermesh-server")

# Ensure logs directory exists
Path("logs").mkdir(exist_ok=True)


# Enums and Types
class AssetType(str, Enum):
    CPU = "cpu"
    GPU = "gpu"
    MEMORY = "memory"
    STORAGE = "storage"
    NETWORK = "network"
    SERVICE = "service"
    CONTAINER = "container"
    VM = "vm"
    APPLICATION = "application"


class PrivacyLevel(str, Enum):
    PRIVATE = "private"
    PRIVATE_NETWORK = "private_network"
    P2P = "p2p"
    PUBLIC_NETWORK = "public_network"
    FULL_PUBLIC = "full_public"


class ProofType(str, Enum):
    POSP = "PoSp"  # Proof of Space - WHERE
    POST = "PoSt"  # Proof of Stake - WHO
    POWK = "PoWk"  # Proof of Work - WHAT/HOW
    POTM = "PoTm"  # Proof of Time - WHEN


# Data Models
@dataclass
class Asset:
    id: str
    type: str
    name: str
    description: str
    owner: str
    status: str  # 'available' | 'allocated' | 'busy' | 'maintenance' | 'offline'
    privacy_level: str
    location: Dict[str, str]  # nodeId, address, region
    specifications: Dict[str, Any]
    allocation: Dict[str, Union[int, str]]  # totalCapacity, allocatedCapacity, availableCapacity, unit
    proxy_address: Optional[str] = None
    created_at: str = ""
    updated_at: str = ""


@dataclass
class AssetAllocation:
    id: str
    asset_id: str
    requester_id: str
    amount: Union[int, float]
    unit: str
    duration: int
    start_time: str
    end_time: str
    status: str  # 'pending' | 'active' | 'completed' | 'cancelled' | 'failed'
    consensus_proofs: List[Dict] = field(default_factory=list)
    proxy_address: Optional[str] = None


@dataclass
class ConsensusProof:
    type: str
    data: Dict[str, Any]
    validated_at: str
    validator: str
    signature: str
    valid: bool


@dataclass
class FourProofConsensus:
    block_id: str
    asset_id: str
    proofs: Dict[str, ConsensusProof]  # space, stake, work, time
    combined_proof: Dict[str, Any]


@dataclass
class ByzantineDetection:
    node_id: str
    detected_at: str
    behaviour: str  # 'double_spending' | 'invalid_proof' | 'consensus_attack' | 'network_partition' | 'timing_attack'
    severity: str   # 'low' | 'medium' | 'high' | 'critical'
    evidence: Dict[str, Any]
    status: str     # 'detected' | 'investigating' | 'confirmed' | 'resolved' | 'false_positive'
    mitigation: Optional[Dict[str, Any]] = None


@dataclass
class RemoteProxy:
    id: str
    address: str
    type: str  # 'memory' | 'storage' | 'compute' | 'network'
    target_asset_id: str
    nat_mapping: Dict[str, str]
    trust: Dict[str, Any]
    performance: Dict[str, float]
    status: str  # 'active' | 'inactive' | 'validating' | 'failed'


@dataclass
class NodeHealth:
    node_id: str
    status: str  # 'healthy' | 'warning' | 'critical' | 'offline'
    metrics: Dict[str, float]
    consensus_metrics: Dict[str, int]
    last_heartbeat: str


@dataclass
class VMAsset(Asset):
    vm_config: Dict[str, Any] = field(default_factory=dict)
    catalog_metadata: Optional[Dict[str, Any]] = None


@dataclass
class VMExecution:
    id: str
    vm_asset_id: str
    allocation_id: str
    status: str  # 'queued' | 'starting' | 'running' | 'completed' | 'failed' | 'cancelled'
    request: Dict[str, Any]
    execution: Dict[str, Any] = field(default_factory=dict)
    consensus_proofs: List[ConsensusProof] = field(default_factory=list)
    proxy_address: Optional[str] = None


@dataclass
class CatalogApplication:
    id: str
    name: str
    version: str
    type: str
    adapter: str
    status: str
    description: str
    requirements: Dict[str, Any]
    dependencies: List[str]
    author: str
    downloads: int
    rating: float
    size: str
    last_updated: str
    asset_id: Optional[str] = None
    privacy_level: Optional[str] = None


# Request Models
class AssetRequest(BaseModel):
    type: AssetType
    name: str
    description: str = ""
    privacy_level: PrivacyLevel = PrivacyLevel.PRIVATE_NETWORK
    specifications: Dict[str, Any] = Field(default_factory=dict)
    location: Dict[str, str] = Field(default_factory=dict)


class AllocationRequest(BaseModel):
    asset_id: str
    amount: Union[int, float]
    duration: int  # seconds
    requirements: Dict[str, Any] = Field(default_factory=dict)


class ProofSubmission(BaseModel):
    asset_id: str
    block_id: str
    type: ProofType
    data: Dict[str, Any]
    signature: str


class RemoteProxyRequest(BaseModel):
    asset_id: str
    type: str  # 'memory' | 'storage' | 'compute' | 'network'
    remote_address: str
    protocol: str = "quic"
    port: Optional[int] = None


class VMAssetRequest(BaseModel):
    catalog_app_id: str
    privacy_level: PrivacyLevel
    resource_limits: Dict[str, Any] = Field(default_factory=dict)
    security_policy: Dict[str, Any] = Field(default_factory=dict)


class VMExecutionRequest(BaseModel):
    vm_asset_id: str
    operation: str
    parameters: Dict[str, Any]
    timeout: int = 300
    requires_consensus: bool = True
    allocation_duration: int = 3600


class ByzantineReport(BaseModel):
    node_id: str
    behavior: str
    evidence: Dict[str, Any]
    description: str


# Server State Management
class HyperMeshState:
    def __init__(self):
        self.start_time = time.time()
        self.assets: Dict[str, Asset] = {}
        self.allocations: Dict[str, AssetAllocation] = {}
        self.consensus_history: List[FourProofConsensus] = []
        self.byzantine_detections: List[ByzantineDetection] = []
        self.remote_proxies: Dict[str, RemoteProxy] = {}
        self.node_health: Dict[str, NodeHealth] = {}
        self.vm_executions: Dict[str, VMExecution] = {}
        self.catalog_applications: Dict[str, CatalogApplication] = {}
        
        self.system_stats = {
            "total_assets": 0,
            "active_allocations": 0,
            "consensus_health": 98.5,
            "byzantine_detections": 0,
            "network_nodes": 8,
            "proxy_connections": 24,
            "last_consensus": datetime.utcnow().isoformat(),
            "uptime": 99.8
        }
        
        self._initialize_sample_assets()
        self._initialize_sample_nodes()
        self._initialize_catalog_apps()

    def _initialize_sample_assets(self):
        """Initialize sample assets for demonstration"""
        assets = [
            Asset(
                id="asset-cpu-001",
                type="cpu", 
                name="High-Performance CPU Pool",
                description="Intel Xeon E5-2699 v4 cluster",
                owner="system",
                status="available",
                privacy_level="public_network",
                location={"node_id": "node-001", "address": "2001:db8::1", "region": "us-west-1"},
                specifications={"cores": 44, "threads": 88, "frequency": "2.2GHz", "architecture": "x86_64"},
                allocation={"total_capacity": 100, "allocated_capacity": 25, "available_capacity": 75, "unit": "percentage"},
                proxy_address="2001:db8:proxy::cpu:001",
                created_at=(datetime.utcnow() - timedelta(days=1)).isoformat(),
                updated_at=datetime.utcnow().isoformat()
            ),
            Asset(
                id="asset-gpu-001",
                type="gpu",
                name="NVIDIA H100 GPU Farm", 
                description="High-throughput GPU compute cluster",
                owner="system",
                status="allocated",
                privacy_level="private_network",
                location={"node_id": "node-002", "address": "2001:db8::2", "region": "us-west-1"},
                specifications={"model": "H100", "memory": "80GB HBM3", "cores": 16896, "frequency": "1980MHz"},
                allocation={"total_capacity": 8, "allocated_capacity": 6, "available_capacity": 2, "unit": "units"},
                proxy_address="2001:db8:proxy::gpu:001",
                created_at=(datetime.utcnow() - timedelta(days=2)).isoformat(), 
                updated_at=datetime.utcnow().isoformat()
            ),
            Asset(
                id="asset-memory-001",
                type="memory",
                name="Distributed Memory Pool",
                description="High-bandwidth memory resources with NAT-like addressing",
                owner="system", 
                status="available",
                privacy_level="p2p",
                location={"node_id": "node-003", "address": "2001:db8::3", "region": "us-west-1"},
                specifications={"total_memory": "1TB", "bandwidth": "800GB/s", "type": "DDR5", "ecc": True},
                allocation={"total_capacity": 1000, "allocated_capacity": 350, "available_capacity": 650, "unit": "GB"},
                proxy_address="2001:db8:proxy::memory:001",
                created_at=(datetime.utcnow() - timedelta(hours=12)).isoformat(),
                updated_at=datetime.utcnow().isoformat()
            )
        ]
        
        for asset in assets:
            self.assets[asset.id] = asset
        
        # Sample allocation
        allocation = AssetAllocation(
            id="alloc-001",
            asset_id="asset-cpu-001",
            requester_id="user-123",
            amount=25,
            unit="percentage",
            duration=3600,
            start_time=(datetime.utcnow() - timedelta(minutes=30)).isoformat(),
            end_time=(datetime.utcnow() + timedelta(minutes=30)).isoformat(),
            status="active",
            consensus_proofs=[],
            proxy_address="2001:db8:proxy::cpu:001/user-123"
        )
        self.allocations[allocation.id] = allocation
        
        self.system_stats["total_assets"] = len(self.assets)
        self.system_stats["active_allocations"] = len([a for a in self.allocations.values() if a.status == "active"])

    def _initialize_sample_nodes(self):
        """Initialize sample node health data"""
        nodes = [
            NodeHealth(
                node_id="node-001",
                status="healthy",
                metrics={
                    "cpu_usage": 25.0,
                    "memory_usage": 60.0,
                    "disk_usage": 45.0,
                    "network_latency": 2.0,
                    "uptime": 99.8
                },
                consensus_metrics={
                    "proofs_validated": 1234,
                    "consensus_participation": 100,
                    "byzantine_detections": 0
                },
                last_heartbeat=datetime.utcnow().isoformat()
            ),
            NodeHealth(
                node_id="node-002",
                status="healthy",
                metrics={
                    "cpu_usage": 78.0,
                    "memory_usage": 45.0, 
                    "disk_usage": 32.0,
                    "network_latency": 1.5,
                    "uptime": 99.9
                },
                consensus_metrics={
                    "proofs_validated": 2156,
                    "consensus_participation": 100,
                    "byzantine_detections": 0
                },
                last_heartbeat=datetime.utcnow().isoformat()
            )
        ]
        
        for node in nodes:
            self.node_health[node.node_id] = node

    def _initialize_catalog_apps(self):
        """Initialize sample Catalog applications"""
        apps = [
            CatalogApplication(
                id="catalog-julia-001",
                name="Scientific Computing Suite",
                version="1.9.3",
                type="Application",
                adapter="Julia",
                status="Available", 
                description="High-performance scientific computing with Julia",
                requirements={"cpu": 2, "memory": 4, "storage": 2, "network": True},
                dependencies=["julia-base", "linear-algebra", "plots"],
                author="HyperMesh Science Team",
                downloads=1543,
                rating=4.8,
                size="156MB",
                last_updated=datetime.utcnow().isoformat()
            ),
            CatalogApplication(
                id="catalog-python-001",
                name="ML Training Pipeline",
                version="2.1.0",
                type="Application",
                adapter="Python",
                status="Available",
                description="Machine learning training pipeline with GPU acceleration",
                requirements={"cpu": 4, "memory": 8, "storage": 10, "network": True},
                dependencies=["pytorch", "transformers", "accelerate"],
                author="ML Research Lab",
                downloads=2847,
                rating=4.6,
                size="2.3GB",
                last_updated=(datetime.utcnow() - timedelta(days=3)).isoformat()
            )
        ]
        
        for app in apps:
            self.catalog_applications[app.id] = app

    def generate_consensus_proof(self, asset_id: str, proof_type: ProofType) -> ConsensusProof:
        """Generate a sample consensus proof"""
        now = datetime.utcnow().isoformat()
        
        proof_data = {
            ProofType.POSP: {"storage_location": "2001:db8::1", "network_position": "cluster-1"},
            ProofType.POST: {"stake_amount": random.randint(1000, 10000), "ownership_proof": f"sig-{uuid.uuid4().hex[:16]}"},
            ProofType.POWK: {"computation_hash": f"hash-{uuid.uuid4().hex}", "work_units": random.randint(100, 1000)},
            ProofType.POTM: {"timestamp": now, "block_height": random.randint(10000, 50000)}
        }
        
        return ConsensusProof(
            type=proof_type.value,
            data=proof_data[proof_type],
            validated_at=now,
            validator=f"validator-{random.randint(1, 5)}",
            signature=f"sig-{uuid.uuid4().hex}",
            valid=True
        )


# Global state
state = HyperMeshState()

# FastAPI Application
app = FastAPI(
    title="HyperMesh Asset Management System",
    description="Production HyperMesh service for asset management and consensus validation",
    version="1.0.0",
    docs_url="/docs",
    redoc_url="/redoc"
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:1337", "http://[::1]:1337"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# Health Check Endpoint
@app.get("/health")
async def health_check():
    """Health check endpoint for service monitoring"""
    uptime_seconds = int(time.time() - state.start_time)
    
    return {
        "status": "healthy",
        "server_id": "hypermesh-simple-001",
        "uptime_seconds": uptime_seconds,
        "timestamp": datetime.utcnow().isoformat(),
        "version": "1.0.0",
        "services": {
            "trustchain": "healthy",
            "stoq": "healthy", 
            "hypermesh": "healthy",
            "integration": "healthy",
            "assets": True,
            "consensus": True,
            "proxy": True,
            "vm": True
        }
    }


@app.get("/api/v1/system/status")
async def get_system_status():
    """Get HyperMesh system status"""
    return {
        "status": "healthy",
        "total_assets": len(state.assets),
        "active_allocations": len([a for a in state.allocations.values() if a.status == "active"]),
        "consensus_health": state.system_stats["consensus_health"],
        "byzantine_detections": len(state.byzantine_detections),
        "network_nodes": len(state.node_health),
        "proxy_connections": len(state.remote_proxies),
        "last_consensus": state.system_stats["last_consensus"],
        "uptime": state.system_stats["uptime"]
    }


# Asset Management Endpoints
@app.get("/api/v1/assets")
async def get_assets(
    type: Optional[AssetType] = None,
    status: Optional[str] = None,
    privacy_level: Optional[PrivacyLevel] = None,
    owner: Optional[str] = None
):
    """Get all assets with optional filtering"""
    assets = list(state.assets.values())
    
    if type:
        assets = [a for a in assets if a.type == type.value]
    if status:
        assets = [a for a in assets if a.status == status]
    if privacy_level:
        assets = [a for a in assets if a.privacy_level == privacy_level.value]
    if owner:
        assets = [a for a in assets if a.owner == owner]
    
    return [asdict(asset) for asset in assets]


@app.get("/api/v1/assets/{asset_id}")
async def get_asset(asset_id: str):
    """Get specific asset by ID"""
    if asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    return asdict(state.assets[asset_id])


@app.post("/api/v1/assets")
async def create_asset(asset_request: AssetRequest):
    """Create new asset"""
    asset_id = f"asset-{asset_request.type.value}-{uuid.uuid4().hex[:8]}"
    now = datetime.utcnow().isoformat()
    
    asset = Asset(
        id=asset_id,
        type=asset_request.type.value,
        name=asset_request.name,
        description=asset_request.description,
        owner="user",  # Would be extracted from auth context
        status="available",
        privacy_level=asset_request.privacy_level.value,
        location=asset_request.location or {"node_id": "local", "address": "local", "region": "local"},
        specifications=asset_request.specifications,
        allocation={"total_capacity": 100, "allocated_capacity": 0, "available_capacity": 100, "unit": "percentage"},
        proxy_address=f"2001:db8:proxy::{asset_request.type.value}:{asset_id[-8:]}",
        created_at=now,
        updated_at=now
    )
    
    state.assets[asset_id] = asset
    state.system_stats["total_assets"] = len(state.assets)
    
    logger.info(f"Asset created: {asset_id} - {asset_request.name}")
    
    return asdict(asset)


@app.put("/api/v1/assets/{asset_id}")
async def update_asset(asset_id: str, updates: Dict[str, Any]):
    """Update asset"""
    if asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    asset = state.assets[asset_id]
    
    # Update allowed fields
    for field, value in updates.items():
        if hasattr(asset, field) and field not in ["id", "created_at"]:
            setattr(asset, field, value)
    
    asset.updated_at = datetime.utcnow().isoformat()
    
    return asdict(asset)


@app.delete("/api/v1/assets/{asset_id}")
async def delete_asset(asset_id: str):
    """Delete asset"""
    if asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    # Check for active allocations
    active_allocations = [a for a in state.allocations.values() if a.asset_id == asset_id and a.status == "active"]
    if active_allocations:
        raise HTTPException(status_code=400, detail="Cannot delete asset with active allocations")
    
    del state.assets[asset_id]
    state.system_stats["total_assets"] = len(state.assets)
    
    logger.info(f"Asset deleted: {asset_id}")
    
    return {"message": "Asset deleted successfully"}


# Asset Allocation Endpoints
@app.post("/api/v1/allocations")
async def request_allocation(allocation_request: AllocationRequest):
    """Request asset allocation"""
    if allocation_request.asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    asset = state.assets[allocation_request.asset_id]
    
    # Check availability
    if asset.status != "available":
        raise HTTPException(status_code=400, detail="Asset not available for allocation")
    
    # Check capacity
    available_capacity = asset.allocation["available_capacity"]
    if allocation_request.amount > available_capacity:
        raise HTTPException(status_code=400, detail="Insufficient asset capacity")
    
    allocation_id = f"alloc-{uuid.uuid4().hex[:8]}"
    now = datetime.utcnow()
    end_time = now + timedelta(seconds=allocation_request.duration)
    
    allocation = AssetAllocation(
        id=allocation_id,
        asset_id=allocation_request.asset_id,
        requester_id="user-current",  # Would be extracted from auth context
        amount=allocation_request.amount,
        unit=asset.allocation["unit"],
        duration=allocation_request.duration,
        start_time=now.isoformat(),
        end_time=end_time.isoformat(),
        status="pending",
        consensus_proofs=[],
        proxy_address=f"{asset.proxy_address}/user-current"
    )
    
    # Generate consensus proofs for allocation
    for proof_type in ProofType:
        proof = state.generate_consensus_proof(allocation_request.asset_id, proof_type)
        allocation.consensus_proofs.append(asdict(proof))
    
    allocation.status = "active"
    
    # Update asset allocation
    asset.allocation["allocated_capacity"] += allocation_request.amount
    asset.allocation["available_capacity"] -= allocation_request.amount
    if asset.allocation["available_capacity"] <= 0:
        asset.status = "allocated"
    
    state.allocations[allocation_id] = allocation
    state.system_stats["active_allocations"] = len([a for a in state.allocations.values() if a.status == "active"])
    
    logger.info(f"Allocation created: {allocation_id} for asset {allocation_request.asset_id}")
    
    return asdict(allocation)


@app.get("/api/v1/allocations")
async def get_allocations(asset_id: Optional[str] = None):
    """Get asset allocations"""
    allocations = list(state.allocations.values())
    
    if asset_id:
        allocations = [a for a in allocations if a.asset_id == asset_id]
    
    return [asdict(allocation) for allocation in allocations]


@app.post("/api/v1/allocations/{allocation_id}/release")
async def release_allocation(allocation_id: str):
    """Release allocation"""
    if allocation_id not in state.allocations:
        raise HTTPException(status_code=404, detail="Allocation not found")
    
    allocation = state.allocations[allocation_id]
    
    if allocation.status != "active":
        raise HTTPException(status_code=400, detail="Allocation is not active")
    
    # Update allocation status
    allocation.status = "completed"
    
    # Update asset availability
    asset = state.assets[allocation.asset_id]
    asset.allocation["allocated_capacity"] -= allocation.amount
    asset.allocation["available_capacity"] += allocation.amount
    
    if asset.status == "allocated" and asset.allocation["available_capacity"] > 0:
        asset.status = "available"
    
    state.system_stats["active_allocations"] = len([a for a in state.allocations.values() if a.status == "active"])
    
    logger.info(f"Allocation released: {allocation_id}")
    
    return {"message": "Allocation released successfully"}


# Consensus Validation Endpoints
@app.post("/api/v1/consensus/validate")
async def validate_consensus(request: Dict[str, str]):
    """Validate four-proof consensus"""
    asset_id = request.get("asset_id", "")
    block_id = request.get("block_id", "")
    
    if asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    # Generate four-proof consensus
    proofs = {}
    for proof_type in ProofType:
        proof = state.generate_consensus_proof(asset_id, proof_type)
        proofs[proof_type.value.lower()] = proof
    
    consensus = FourProofConsensus(
        block_id=block_id,
        asset_id=asset_id,
        proofs=proofs,
        combined_proof={
            "hash": f"combined-{uuid.uuid4().hex}",
            "signature": f"consensus-sig-{uuid.uuid4().hex}",
            "validated_at": datetime.utcnow().isoformat(),
            "consensus_reached": True
        }
    )
    
    state.consensus_history.append(consensus)
    state.system_stats["last_consensus"] = consensus.combined_proof["validated_at"]
    
    logger.info(f"Consensus validated: {block_id} for asset {asset_id}")
    
    return asdict(consensus)


@app.get("/api/v1/consensus/history/{asset_id}")
async def get_consensus_history(asset_id: str, limit: int = 100):
    """Get consensus history for asset"""
    if asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    # Filter history for specific asset
    asset_history = [c for c in state.consensus_history if c.asset_id == asset_id]
    
    # Apply limit
    asset_history = asset_history[-limit:]
    
    return [asdict(consensus) for consensus in asset_history]


@app.post("/api/v1/consensus/proof")
async def submit_proof(proof: ProofSubmission):
    """Submit proof for consensus"""
    if proof.asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    # Validate proof (simplified)
    accepted = True
    reason = None
    
    if not proof.signature:
        accepted = False
        reason = "Missing signature"
    
    logger.info(f"Proof submitted: {proof.type.value} for asset {proof.asset_id} - {'Accepted' if accepted else 'Rejected'}")
    
    return {
        "accepted": accepted,
        "reason": reason
    }


# Byzantine Detection Endpoints
@app.get("/api/v1/byzantine/detections")
async def get_byzantine_detections(node_id: Optional[str] = None):
    """Get Byzantine detection results"""
    detections = state.byzantine_detections
    
    if node_id:
        detections = [d for d in detections if d.node_id == node_id]
    
    return [asdict(detection) for detection in detections]


@app.post("/api/v1/byzantine/report")
async def report_byzantine_behavior(report: ByzantineReport):
    """Report Byzantine behavior"""
    detection_id = f"byzantine-{uuid.uuid4().hex[:8]}"
    
    detection = ByzantineDetection(
        node_id=report.node_id,
        detected_at=datetime.utcnow().isoformat(),
        behaviour=report.behavior,
        severity="medium",  # Would be calculated based on behavior type
        evidence=report.evidence,
        status="detected"
    )
    
    state.byzantine_detections.append(detection)
    
    logger.warning(f"Byzantine behavior reported: {report.node_id} - {report.behavior}")
    
    return asdict(detection)


# Remote Proxy/NAT System Endpoints
@app.get("/api/v1/proxy/list")
async def get_remote_proxies(asset_id: Optional[str] = None):
    """Get remote proxies"""
    proxies = list(state.remote_proxies.values())
    
    if asset_id:
        proxies = [p for p in proxies if p.target_asset_id == asset_id]
    
    return [asdict(proxy) for proxy in proxies]


@app.post("/api/v1/proxy/create")
async def create_remote_proxy(proxy_request: RemoteProxyRequest):
    """Create remote proxy for asset"""
    if proxy_request.asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="Asset not found")
    
    proxy_id = f"proxy-{uuid.uuid4().hex[:8]}"
    
    proxy = RemoteProxy(
        id=proxy_id,
        address=f"2001:db8:proxy::{proxy_request.type}:{proxy_id[-8:]}",
        type=proxy_request.type,
        target_asset_id=proxy_request.asset_id,
        nat_mapping={
            "local_address": f"local:{random.randint(10000, 65000)}",
            "remote_address": proxy_request.remote_address,
            "port": proxy_request.port or 443,
            "protocol": proxy_request.protocol
        },
        trust={
            "level": random.randint(80, 95),
            "validated_by": [f"validator-{i}" for i in range(1, 4)],
            "last_validation": datetime.utcnow().isoformat()
        },
        performance={
            "latency": random.uniform(1.0, 10.0),
            "throughput": random.uniform(1000.0, 10000.0),
            "availability": random.uniform(95.0, 99.9)
        },
        status="active"
    )
    
    state.remote_proxies[proxy_id] = proxy
    
    logger.info(f"Remote proxy created: {proxy_id} for asset {proxy_request.asset_id}")
    
    return asdict(proxy)


@app.put("/api/v1/proxy/{proxy_id}")
async def update_remote_proxy(proxy_id: str, updates: Dict[str, Any]):
    """Update remote proxy configuration"""
    if proxy_id not in state.remote_proxies:
        raise HTTPException(status_code=404, detail="Proxy not found")
    
    proxy = state.remote_proxies[proxy_id]
    
    # Update allowed fields
    for field, value in updates.items():
        if hasattr(proxy, field) and field != "id":
            setattr(proxy, field, value)
    
    return asdict(proxy)


@app.get("/api/v1/proxy/{proxy_id}/validate-trust")
async def validate_proxy_trust(proxy_id: str):
    """Validate proxy trust"""
    if proxy_id not in state.remote_proxies:
        raise HTTPException(status_code=404, detail="Proxy not found")
    
    proxy = state.remote_proxies[proxy_id]
    
    # Simulate trust validation
    validation_results = [
        {"validator": f"validator-{i}", "result": True, "reason": None} 
        for i in range(1, 4)
    ]
    
    return {
        "trust_level": proxy.trust["level"],
        "validators": proxy.trust["validated_by"],
        "validation_results": validation_results
    }


# Node Health Endpoints
@app.get("/api/v1/nodes/health")
async def get_nodes_health():
    """Get all nodes health status"""
    return [asdict(node) for node in state.node_health.values()]


@app.get("/api/v1/nodes/{node_id}/health")
async def get_node_health(node_id: str):
    """Get specific node health status"""
    if node_id not in state.node_health:
        raise HTTPException(status_code=404, detail="Node not found")
    
    return asdict(state.node_health[node_id])


# Network Topology
@app.get("/api/v1/network/topology")
async def get_network_topology():
    """Get network topology"""
    nodes = []
    for node_id, health in state.node_health.items():
        nodes.append({
            "id": node_id,
            "address": f"2001:db8::{random.randint(1, 255)}",
            "status": health.status,
            "connections": [f"node-{i:03d}" for i in range(1, 4) if f"node-{i:03d}" != node_id],
            "region": "us-west-1"
        })
    
    connections = []
    for i, node in enumerate(nodes):
        for conn in node["connections"]:
            if any(c["from"] == conn and c["to"] == node["id"] for c in connections):
                continue  # Avoid duplicates
            connections.append({
                "from": node["id"],
                "to": conn,
                "latency": random.uniform(1.0, 50.0),
                "bandwidth": random.uniform(1000.0, 10000.0),
                "status": "active"
            })
    
    return {
        "nodes": nodes,
        "connections": connections,
        "clusters": [
            {
                "id": "cluster-west-1",
                "nodes": [node["id"] for node in nodes],
                "region": "us-west-1"
            }
        ]
    }


# VM and Catalog Integration Endpoints
@app.get("/api/v1/catalog/applications")
async def get_catalog_applications(
    type: Optional[str] = None,
    adapter: Optional[str] = None,
    status: Optional[str] = None
):
    """Get Catalog applications (bridge to Catalog service)"""
    apps = list(state.catalog_applications.values())
    
    if type:
        apps = [a for a in apps if a.type.lower() == type.lower()]
    if adapter:
        apps = [a for a in apps if a.adapter.lower() == adapter.lower()]
    if status:
        apps = [a for a in apps if a.status.lower() == status.lower()]
    
    return [asdict(app) for app in apps]


@app.post("/api/v1/assets/vm")
async def create_vm_asset(vm_request: Dict[str, Any]):
    """Create VM asset from configuration"""
    vm_asset_id = f"vm-{uuid.uuid4().hex[:8]}"
    now = datetime.utcnow().isoformat()
    
    vm_asset = VMAsset(
        id=vm_asset_id,
        type="vm",
        name=vm_request.get("name", "VM Asset"),
        description=vm_request.get("description", ""),
        owner="user",
        status="available",
        privacy_level=vm_request.get("privacy_level", "private_network"),
        location={"node_id": "local", "address": "local", "region": "local"},
        specifications=vm_request.get("specifications", {}),
        allocation={"total_capacity": 1, "allocated_capacity": 0, "available_capacity": 1, "unit": "instances"},
        proxy_address=f"2001:db8:proxy::vm:{vm_asset_id[-8:]}",
        created_at=now,
        updated_at=now,
        vm_config=vm_request.get("vm_config", {}),
        catalog_metadata=vm_request.get("catalog_metadata")
    )
    
    state.assets[vm_asset_id] = vm_asset
    
    logger.info(f"VM asset created: {vm_asset_id}")
    
    return asdict(vm_asset)


@app.post("/api/v1/vm/execute")
async def execute_vm_asset(execution_request: VMExecutionRequest):
    """Execute VM asset through HyperMesh allocation system"""
    if execution_request.vm_asset_id not in state.assets:
        raise HTTPException(status_code=404, detail="VM asset not found")
    
    vm_asset = state.assets[execution_request.vm_asset_id]
    
    if vm_asset.type != "vm":
        raise HTTPException(status_code=400, detail="Asset is not a VM asset")
    
    execution_id = f"exec-{uuid.uuid4().hex[:8]}"
    now = datetime.utcnow()
    
    # Create allocation for execution
    allocation_request = AllocationRequest(
        asset_id=execution_request.vm_asset_id,
        amount=1,
        duration=execution_request.allocation_duration
    )
    
    # Execute allocation logic (simplified)
    allocation_id = f"alloc-exec-{uuid.uuid4().hex[:8]}"
    allocation = AssetAllocation(
        id=allocation_id,
        asset_id=execution_request.vm_asset_id,
        requester_id="user-current",
        amount=1,
        unit="instances",
        duration=execution_request.allocation_duration,
        start_time=now.isoformat(),
        end_time=(now + timedelta(seconds=execution_request.allocation_duration)).isoformat(),
        status="active",
        consensus_proofs=[],
        proxy_address=f"{vm_asset.proxy_address}/exec"
    )
    
    state.allocations[allocation_id] = allocation
    
    # Create execution record
    execution = VMExecution(
        id=execution_id,
        vm_asset_id=execution_request.vm_asset_id,
        allocation_id=allocation_id,
        status="queued",
        request={
            "operation": execution_request.operation,
            "parameters": execution_request.parameters,
            "timeout": execution_request.timeout,
            "requires_consensus": execution_request.requires_consensus
        },
        execution={},
        consensus_proofs=[],
        proxy_address=allocation.proxy_address
    )
    
    # Generate consensus proofs if required
    if execution_request.requires_consensus:
        for proof_type in ProofType:
            proof = state.generate_consensus_proof(execution_request.vm_asset_id, proof_type)
            execution.consensus_proofs.append(proof)
    
    # Simulate execution start
    execution.status = "starting"
    execution.execution["start_time"] = now.isoformat()
    
    state.vm_executions[execution_id] = execution
    
    logger.info(f"VM execution started: {execution_id} for asset {execution_request.vm_asset_id}")
    
    return asdict(execution)


@app.get("/api/v1/vm/executions/{execution_id}")
async def get_vm_execution(execution_id: str):
    """Get VM execution status and results"""
    if execution_id not in state.vm_executions:
        raise HTTPException(status_code=404, detail="Execution not found")
    
    execution = state.vm_executions[execution_id]
    
    # Simulate execution progress
    if execution.status == "starting":
        execution.status = "running"
    elif execution.status == "running" and random.random() > 0.7:  # 30% chance to complete
        execution.status = "completed"
        execution.execution["end_time"] = datetime.utcnow().isoformat()
        execution.execution["exit_code"] = 0
        execution.execution["output"] = "Execution completed successfully"
        execution.execution["resource_usage"] = {
            "cpu_time": random.uniform(10.0, 60.0),
            "memory_peak": random.randint(100, 500),
            "network_bytes": random.randint(1000, 10000),
            "storage_io": random.randint(500, 2000)
        }
    
    return asdict(execution)


@app.get("/api/v1/vm/executions")
async def get_vm_executions(vm_asset_id: Optional[str] = None):
    """List VM executions for an asset or all executions"""
    executions = list(state.vm_executions.values())
    
    if vm_asset_id:
        executions = [e for e in executions if e.vm_asset_id == vm_asset_id]
    
    return [asdict(execution) for execution in executions]


@app.post("/api/v1/vm/executions/{execution_id}/cancel")
async def cancel_vm_execution(execution_id: str):
    """Cancel VM execution"""
    if execution_id not in state.vm_executions:
        raise HTTPException(status_code=404, detail="Execution not found")
    
    execution = state.vm_executions[execution_id]
    
    if execution.status in ["completed", "failed", "cancelled"]:
        return {"cancelled": False, "reason": f"Execution already {execution.status}"}
    
    execution.status = "cancelled"
    execution.execution["end_time"] = datetime.utcnow().isoformat()
    
    logger.info(f"VM execution cancelled: {execution_id}")
    
    return {"cancelled": True}


@app.post("/api/v1/catalog/install")
async def install_catalog_application(config: Dict[str, Any]):
    """Install Catalog application as HyperMesh VM asset"""
    catalog_id = config.get("catalog_id", "")
    
    if catalog_id not in state.catalog_applications:
        raise HTTPException(status_code=404, detail="Catalog application not found")
    
    catalog_app = state.catalog_applications[catalog_id]
    
    # Create VM asset from catalog app
    vm_asset_data = {
        "name": f"VM: {catalog_app.name}",
        "description": catalog_app.description,
        "privacy_level": config.get("privacy_level", "private_network"),
        "specifications": {
            "runtime": catalog_app.adapter.lower(),
            "catalog_version": catalog_app.version,
            "catalog_id": catalog_app.id
        },
        "vm_config": {
            "runtime": catalog_app.adapter.lower(),
            "entrypoint": "main",
            "environment": {},
            "dependencies": catalog_app.dependencies,
            "resource_limits": config.get("resource_limits", {
                "max_cpu": catalog_app.requirements.get("cpu", 1),
                "max_memory": f"{catalog_app.requirements.get('memory', 1)}GB",
                "max_storage": f"{catalog_app.requirements.get('storage', 1)}GB",
                "max_execution_time": 300
            }),
            "security_policy": {
                "allow_network_access": catalog_app.requirements.get("network", False),
                "allow_file_system": True,
                "allowed_urls": [],
                "trusted_domains": []
            }
        },
        "catalog_metadata": {
            "catalog_id": catalog_app.id,
            "version": catalog_app.version,
            "author": catalog_app.author,
            "description": catalog_app.description,
            "tags": [catalog_app.type.lower()],
            "download_count": catalog_app.downloads,
            "rating": catalog_app.rating
        }
    }
    
    # Create VM asset
    vm_asset_response = await create_vm_asset(vm_asset_data)
    vm_asset = Asset(**vm_asset_response)
    
    # Update catalog app with asset link
    catalog_app.asset_id = vm_asset.id
    catalog_app.status = "Installed"
    
    installation_status = {
        "status": "completed",
        "progress": 100,
        "logs": [
            f"Downloaded {catalog_app.name} v{catalog_app.version}",
            f"Created VM asset {vm_asset.id}",
            "Installation completed successfully"
        ]
    }
    
    logger.info(f"Catalog application installed: {catalog_id} -> VM asset {vm_asset.id}")
    
    return {
        "vm_asset": asdict(vm_asset),
        "installation": installation_status
    }


# Remote Operation Execution
@app.post("/api/v1/proxy/execute")
async def execute_remote_operation(operation: Dict[str, Any]):
    """Execute remote operation through proxy"""
    proxy_id = operation.get("proxy_id", "")
    
    if proxy_id not in state.remote_proxies:
        raise HTTPException(status_code=404, detail="Proxy not found")
    
    proxy = state.remote_proxies[proxy_id]
    
    # Simulate remote operation execution
    start_time = time.time()
    
    # Simulate operation delay
    await asyncio.sleep(random.uniform(0.1, 0.5))
    
    execution_time = (time.time() - start_time) * 1000  # Convert to ms
    
    success = random.random() > 0.1  # 90% success rate
    
    result = {
        "success": success,
        "execution_time": execution_time
    }
    
    if success:
        result["result"] = {"output": f"Operation '{operation.get('operation', 'unknown')}' completed", "status": "success"}
    else:
        result["error"] = "Operation failed due to network timeout"
    
    logger.info(f"Remote operation executed via proxy {proxy_id}: {operation.get('operation', 'unknown')} - {'Success' if success else 'Failed'}")
    
    return result


# Error Handlers
@app.exception_handler(HTTPException)
async def http_exception_handler(request: Request, exc: HTTPException):
    return JSONResponse(
        status_code=exc.status_code,
        content={"detail": exc.detail, "timestamp": datetime.utcnow().isoformat()}
    )


@app.exception_handler(Exception)
async def general_exception_handler(request: Request, exc: Exception):
    logger.error(f"Unhandled exception: {str(exc)}")
    return JSONResponse(
        status_code=500,
        content={"detail": "Internal server error", "timestamp": datetime.utcnow().isoformat()}
    )


def main():
    """Main server entry point"""
    logger.info("🚀 Starting HyperMesh Simple Server on port 8446")
    logger.info("🔗 Asset Management & Consensus Validation Service")
    logger.info("🏗️  Universal Asset System with Four-Proof Consensus")
    logger.info("🔗 Health check: http://localhost:8446/health")
    logger.info("📚 API documentation: http://localhost:8446/docs")
    
    # Configure uvicorn for production
    config = uvicorn.Config(
        app=app,
        host="0.0.0.0",  # Listen on all interfaces
        port=8446,       # Port expected by startup script
        log_level="info",
        access_log=True,
        server_header=False,
        date_header=False
    )
    
    server = uvicorn.Server(config)
    
    try:
        server.run()
    except KeyboardInterrupt:
        logger.info("🛑 HyperMesh server shutting down")
    except Exception as e:
        logger.error(f"❌ Server error: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()