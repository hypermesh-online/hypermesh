"""HyperMesh Python SDK.

Wraps the HyperMesh node HTTP REST API with typed dataclass responses
and both sync (urllib) and async (httpx) transports.

Usage::

    from hypermesh import HyperMeshClient

    # Sync (zero dependencies)
    client = HyperMeshClient()
    status = client.node.status()
    print(status.node_id, status.chain_height)

    # Async (requires httpx)
    async_client = HyperMeshClient(async_mode=True)
    status = await async_client.node.status()
"""

from __future__ import annotations

from .client import AsyncTransport, SyncTransport
from .errors import ApiError, ConnectionError, HyperMeshError, NotFoundError
from .types import (
    Asset,
    AssetList,
    Block,
    BlockchainHeight,
    CaesarBalance,
    CaesarGovernorParams,
    CaesarRewardInfo,
    CaesarRouteResult,
    CaesarTransactionList,
    CaesarWalletInfo,
    CatalogPackageInfo,
    CatalogPackageList,
    CatalogRegistryStats,
    CatalogSearchResults,
    Dashboard,
    DashboardInfo,
    DashboardList,
    DnsList,
    DnsRecord,
    Domain,
    DomainList,
    EngaugeCapacityMetrics,
    EngaugeLeaseList,
    EngaugeListingList,
    EngaugeNodeMetrics,
    EngaugeTrafficMetrics,
    Neighbor,
    Neighbors,
    NodeStatus,
    Peer,
    PeerList,
    TopologyInfo,
    TrustChainCertificate,
    TrustChainCertificateList,
    TrustChainDnsZoneList,
    TrustChainRevokeResult,
    TrustChainValidationResult,
    ValidationResult,
)

_DEFAULT_BASE_URL = "http://localhost:9293"
_DEFAULT_CAESAR_URL = "http://localhost:9294"
_DEFAULT_TRUSTCHAIN_URL = "http://localhost:8444"
_DEFAULT_CATALOG_URL = "http://localhost:9295"
_DEFAULT_ENGAUGE_URL = "http://localhost:9296"


class HyperMeshClient:
    """Top-level client for the HyperMesh node API.

    Parameters
    ----------
    base_url:
        Node HTTP address. Default ``http://localhost:9293``.
    caesar_url:
        Caesar EVP service address. Default ``http://localhost:9294``.
    trustchain_url:
        TrustChain service address. Default ``http://localhost:8444``.
    catalog_url:
        Catalog service address. Default ``http://localhost:9295``.
    engauge_url:
        Engauge service address. Default ``http://localhost:9296``.
    async_mode:
        When ``True``, use httpx-based async transport. All API
        methods become coroutines.
    timeout:
        Request timeout in seconds.
    """

    def __init__(
        self,
        base_url: str = _DEFAULT_BASE_URL,
        *,
        caesar_url: str = _DEFAULT_CAESAR_URL,
        trustchain_url: str = _DEFAULT_TRUSTCHAIN_URL,
        catalog_url: str = _DEFAULT_CATALOG_URL,
        engauge_url: str = _DEFAULT_ENGAUGE_URL,
        async_mode: bool = False,
        timeout: float = 30.0,
    ) -> None:
        self._base_url = base_url
        self._async_mode = async_mode

        if async_mode:
            transport = AsyncTransport(base_url, timeout=timeout)
            caesar_transport = AsyncTransport(caesar_url, timeout=timeout)
            trustchain_transport = AsyncTransport(trustchain_url, timeout=timeout)
            catalog_transport = AsyncTransport(catalog_url, timeout=timeout)
            engauge_transport = AsyncTransport(engauge_url, timeout=timeout)
            self._async_transport = transport
            self._async_service_transports = [
                caesar_transport,
                trustchain_transport,
                catalog_transport,
                engauge_transport,
            ]

            from .api.node import AsyncNodeApi
            from .api.blockchain import AsyncBlockchainApi
            from .api.dns import AsyncDnsApi
            from .api.network import AsyncNetworkApi
            from .api.topology import AsyncTopologyApi
            from .api.asset import AsyncAssetApi
            from .api.dashboard import AsyncDashboardApi
            from .api.config import AsyncConfigApi
            from .api.domain import AsyncDomainApi
            from .api.caesar import AsyncCaesarApi
            from .api.trustchain import AsyncTrustChainApi
            from .api.engauge import AsyncEngaugeApi
            from .api.catalog import AsyncCatalogApi

            self.node = AsyncNodeApi(transport)
            self.blockchain = AsyncBlockchainApi(transport)
            self.dns = AsyncDnsApi(transport)
            self.network = AsyncNetworkApi(transport)
            self.topology = AsyncTopologyApi(transport)
            self.asset = AsyncAssetApi(transport)
            self.dashboard = AsyncDashboardApi(transport)
            self.config = AsyncConfigApi(transport)
            self.domain = AsyncDomainApi(transport)
            self.caesar = AsyncCaesarApi(caesar_transport)
            self.trustchain = AsyncTrustChainApi(trustchain_transport)
            self.engauge = AsyncEngaugeApi(engauge_transport)
            self.catalog = AsyncCatalogApi(catalog_transport)
        else:
            transport = SyncTransport(base_url, timeout=timeout)  # type: ignore[assignment]
            self._async_transport = None
            self._async_service_transports = []

            from .api.node import NodeApi
            from .api.blockchain import BlockchainApi
            from .api.dns import DnsApi
            from .api.network import NetworkApi
            from .api.topology import TopologyApi
            from .api.asset import AssetApi
            from .api.dashboard import DashboardApi
            from .api.config import ConfigApi
            from .api.domain import DomainApi
            from .api.caesar import CaesarApi
            from .api.trustchain import TrustChainApi
            from .api.engauge import EngaugeApi
            from .api.catalog import CatalogApi

            self.node = NodeApi(transport)
            self.blockchain = BlockchainApi(transport)
            self.dns = DnsApi(transport)
            self.network = NetworkApi(transport)
            self.topology = TopologyApi(transport)
            self.asset = AssetApi(transport)
            self.dashboard = DashboardApi(transport)
            self.config = ConfigApi(transport)
            self.domain = DomainApi(transport)
            self.caesar = CaesarApi(
                SyncTransport(caesar_url, timeout=timeout)
            )
            self.trustchain = TrustChainApi(
                SyncTransport(trustchain_url, timeout=timeout)
            )
            self.engauge = EngaugeApi(
                SyncTransport(engauge_url, timeout=timeout)
            )
            self.catalog = CatalogApi(
                SyncTransport(catalog_url, timeout=timeout)
            )

    async def close(self) -> None:
        """Close all async transports. No-op for sync clients."""
        if self._async_transport is not None:
            await self._async_transport.close()
        for t in self._async_service_transports:
            await t.close()

    def __repr__(self) -> str:
        mode = "async" if self._async_mode else "sync"
        return f"HyperMeshClient({self._base_url!r}, mode={mode})"


__all__ = [
    "HyperMeshClient",
    "HyperMeshError",
    "ApiError",
    "ConnectionError",
    "NotFoundError",
    "NodeStatus",
    "BlockchainHeight",
    "Block",
    "ValidationResult",
    "DnsRecord",
    "DnsList",
    "Peer",
    "PeerList",
    "TopologyInfo",
    "Neighbor",
    "Neighbors",
    "Asset",
    "AssetList",
    "Dashboard",
    "DashboardList",
    "DashboardInfo",
    "Domain",
    "DomainList",
    "CaesarWalletInfo",
    "CaesarBalance",
    "CaesarTransactionList",
    "CaesarRewardInfo",
    "CaesarRouteResult",
    "CaesarGovernorParams",
    "TrustChainCertificate",
    "TrustChainCertificateList",
    "TrustChainValidationResult",
    "TrustChainRevokeResult",
    "TrustChainDnsZoneList",
    "EngaugeCapacityMetrics",
    "EngaugeTrafficMetrics",
    "EngaugeListingList",
    "EngaugeNodeMetrics",
    "EngaugeLeaseList",
    "CatalogPackageInfo",
    "CatalogPackageList",
    "CatalogSearchResults",
    "CatalogRegistryStats",
]
