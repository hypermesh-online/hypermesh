"""Dataclass response types for the HyperMesh API."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class NodeStatus:
    """Response from GET /api/v1/status."""

    chain_height: int
    coordinate: dict[str, Any]
    node_id: str
    peers: int
    privacy_mode: str
    uptime_secs: float


@dataclass
class BlockchainHeight:
    """Response from GET /api/v1/blockchain/height."""

    height: int


@dataclass
class Block:
    """A single block from the blockchain."""

    index: int
    timestamp: str
    data: dict[str, Any] = field(default_factory=dict)
    hash: str = ""
    previous_hash: str = ""


@dataclass
class ValidationResult:
    """Response from GET /api/v1/blockchain/validate."""

    valid: bool
    errors: list[str] = field(default_factory=list)


@dataclass
class DnsRecord:
    """A single DNS record."""

    name: str
    address: str


@dataclass
class DnsList:
    """Response from GET /api/v1/dns/list."""

    count: int
    records: list[DnsRecord]


@dataclass
class Peer:
    """A connected peer."""

    node_id: str
    address: str
    connected_at: str = ""


@dataclass
class PeerList:
    """Response from GET /api/v1/network/peers."""

    count: int
    peers: list[Peer]


@dataclass
class TopologyInfo:
    """Response from GET /api/v1/topology/info."""

    coordinate: dict[str, Any]
    node_id: str


@dataclass
class Neighbor:
    """A topology neighbor."""

    node_id: str
    coordinate: dict[str, Any]
    distance: float = 0.0


@dataclass
class Neighbors:
    """Response from GET /api/v1/topology/neighbors."""

    center: dict[str, Any]
    count: int
    neighbors: list[Neighbor]
    radius: float


@dataclass
class Asset:
    """A registered asset."""

    asset_id: str
    asset_type: str
    state: str = ""
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class AssetList:
    """Response from GET /api/v1/asset/list."""

    count: int
    assets: list[Asset]


@dataclass
class Dashboard:
    """A dashboard entry."""

    name: str
    scope: str = ""
    url: str = ""


@dataclass
class DashboardList:
    """Response from GET /api/v1/dashboard/list."""

    count: int
    dashboards: list[Dashboard]


@dataclass
class DashboardInfo:
    """Response from GET /api/v1/dashboard/info."""

    name: str
    version: str = ""
    scope: str = ""
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class Domain:
    """A registered domain."""

    name: str
    privacy: str = ""
    owner: str = ""


@dataclass
class DomainList:
    """Response from GET /api/v1/domain/list."""

    count: int
    domains: list[Domain]
