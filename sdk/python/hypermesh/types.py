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


# ── Caesar ──


@dataclass
class CaesarWalletInfo:
    """Response from GET /api/v1/caesar/wallet."""

    balance_grams: float
    balance_usd: float
    tier: str
    node_id: str


@dataclass
class CaesarBalance:
    """Response from GET /api/v1/caesar/balance."""

    gold_grams: float
    usd_equivalent: float
    tier: str


@dataclass
class CaesarTransactionList:
    """Response from GET /api/v1/caesar/transactions."""

    count: int
    transactions: list[dict[str, Any]]


@dataclass
class CaesarRewardInfo:
    """Response from GET /api/v1/caesar/rewards."""

    total_earned: float
    pending: float
    tier_multiplier: float


@dataclass
class CaesarRouteResult:
    """Response from POST /api/v1/caesar/route."""

    packet_id: str
    status: str
    fee: float


@dataclass
class CaesarGovernorParams:
    """Response from GET /api/v1/caesar/governor/params."""

    velocity: float
    fee_rate: float
    demurrage_rate: float


# ── TrustChain ──


@dataclass
class TrustChainCertificate:
    """A TrustChain certificate."""

    id: str
    subject: str
    scope: str
    valid_from: str
    valid_to: str
    pem: str


@dataclass
class TrustChainCertificateList:
    """Response from GET /api/v1/trustchain/certificates."""

    count: int
    certificates: list[TrustChainCertificate]


@dataclass
class TrustChainValidationResult:
    """Response from POST /api/v1/trustchain/validate."""

    valid: bool
    errors: list[str] = field(default_factory=list)
    chain_valid: bool = False


@dataclass
class TrustChainRevokeResult:
    """Response from POST /api/v1/trustchain/revoke."""

    revoked: bool
    cert_id: str


@dataclass
class TrustChainDnsZoneList:
    """Response from GET /api/v1/trustchain/dns/zones."""

    count: int
    zones: list[dict[str, Any]]


# ── NGauge ──


@dataclass
class NGaugeCapacityMetrics:
    """Response from GET /api/v1/ngauge/capacity."""

    bytes_served: int
    compute_delivered: float
    storage: int
    bandwidth: float
    uptime: float


@dataclass
class NGaugeTrafficMetrics:
    """Response from GET /api/v1/ngauge/traffic."""

    organic_ratio: float
    speculative_ratio: float
    total_requests: int


@dataclass
class NGaugeListingList:
    """Response from GET /api/v1/ngauge/marketplace/listings."""

    count: int
    listings: list[dict[str, Any]]


@dataclass
class NGaugeNodeMetrics:
    """Response from GET /api/v1/ngauge/node/metrics."""

    activity_score: float
    receipts: int
    bandwidth: float


@dataclass
class NGaugeLeaseList:
    """Response from GET /api/v1/ngauge/leases."""

    count: int
    leases: list[dict[str, Any]]


# ── Catalog ──


@dataclass
class CatalogPackageInfo:
    """A catalog package."""

    name: str
    version: str
    description: str
    author: str
    downloads: int


@dataclass
class CatalogPackageList:
    """Response from GET /api/v1/catalog/browse."""

    count: int
    packages: list[CatalogPackageInfo]


@dataclass
class CatalogSearchResults:
    """Response from GET /api/v1/catalog/search."""

    count: int
    results: list[dict[str, Any]]


@dataclass
class CatalogRegistryStats:
    """Response from GET /api/v1/catalog/registry/stats."""

    package_count: int
    publisher_count: int
    total_downloads: int
