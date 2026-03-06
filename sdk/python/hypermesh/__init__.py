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
    Dashboard,
    DashboardInfo,
    DashboardList,
    DnsList,
    DnsRecord,
    Domain,
    DomainList,
    Neighbor,
    Neighbors,
    NodeStatus,
    Peer,
    PeerList,
    TopologyInfo,
    ValidationResult,
)

_DEFAULT_BASE_URL = "http://localhost:9293"


class HyperMeshClient:
    """Top-level client for the HyperMesh node API.

    Parameters
    ----------
    base_url:
        Node HTTP address. Default ``http://localhost:9293``.
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
        async_mode: bool = False,
        timeout: float = 30.0,
    ) -> None:
        self._base_url = base_url
        self._async_mode = async_mode

        if async_mode:
            transport = AsyncTransport(base_url, timeout=timeout)
            self._async_transport = transport
            from .api.node import AsyncNodeApi
            from .api.blockchain import AsyncBlockchainApi
            from .api.dns import AsyncDnsApi
            from .api.network import AsyncNetworkApi
            from .api.topology import AsyncTopologyApi
            from .api.asset import AsyncAssetApi
            from .api.dashboard import AsyncDashboardApi
            from .api.config import AsyncConfigApi
            from .api.domain import AsyncDomainApi

            self.node = AsyncNodeApi(transport)
            self.blockchain = AsyncBlockchainApi(transport)
            self.dns = AsyncDnsApi(transport)
            self.network = AsyncNetworkApi(transport)
            self.topology = AsyncTopologyApi(transport)
            self.asset = AsyncAssetApi(transport)
            self.dashboard = AsyncDashboardApi(transport)
            self.config = AsyncConfigApi(transport)
            self.domain = AsyncDomainApi(transport)
        else:
            transport = SyncTransport(base_url, timeout=timeout)  # type: ignore[assignment]
            self._async_transport = None
            from .api.node import NodeApi
            from .api.blockchain import BlockchainApi
            from .api.dns import DnsApi
            from .api.network import NetworkApi
            from .api.topology import TopologyApi
            from .api.asset import AssetApi
            from .api.dashboard import DashboardApi
            from .api.config import ConfigApi
            from .api.domain import DomainApi

            self.node = NodeApi(transport)
            self.blockchain = BlockchainApi(transport)
            self.dns = DnsApi(transport)
            self.network = NetworkApi(transport)
            self.topology = TopologyApi(transport)
            self.asset = AssetApi(transport)
            self.dashboard = DashboardApi(transport)
            self.config = ConfigApi(transport)
            self.domain = DomainApi(transport)

    async def close(self) -> None:
        """Close the async transport. No-op for sync clients."""
        if self._async_transport is not None:
            await self._async_transport.close()

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
]
