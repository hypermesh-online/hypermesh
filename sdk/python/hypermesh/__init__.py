"""HyperMesh Python SDK.

Wraps the HyperMesh node HTTP REST API with typed dataclass responses
and both sync (urllib) and async (httpx) transports.

For local development with the native shared library, use
:class:`HyperMeshFFI` instead of :class:`HyperMeshClient`.

Usage::

    from hypermesh import HyperMeshClient

    # Sync (zero dependencies)
    client = HyperMeshClient()
    status = client.node.status()
    print(status.node_id, status.chain_height)

    # Async (requires httpx)
    async_client = HyperMeshClient(async_mode=True)
    status = await async_client.node.status()

    # Native FFI (requires libhypermesh_ffi)
    from hypermesh import HyperMeshFFI
    with HyperMeshFFI() as hm:
        print(hm.status())
"""

from __future__ import annotations

from typing import Any

from .client import (
    AsyncTransport,
    CAPABILITY_TOKEN_HEADER,
    SyncTransport,
)
from .errors import ApiError, ConnectionError, HyperMeshError, NotFoundError
from .ffi import FFIError, HyperMeshFFI, LibraryNotFoundError
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
    NGaugeCapacityMetrics,
    NGaugeLeaseList,
    NGaugeListingList,
    NGaugeNodeMetrics,
    NGaugeTrafficMetrics,
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

_DEFAULT_BASE_URL = "https://localhost:8443"
_DEFAULT_CAESAR_URL = "https://localhost:8443"
_DEFAULT_TRUSTCHAIN_URL = "https://localhost:8443"
_DEFAULT_CATALOG_URL = "https://localhost:8443"
_DEFAULT_NGAUGE_URL = "https://localhost:8443"


class HyperMeshClient:
    """Top-level client for the HyperMesh node API.

    Parameters
    ----------
    base_url:
        Gateway HTTP/3 address. Default ``https://localhost:8443``.
    caesar_url:
        Caesar EVP service address. Default ``https://localhost:8443``.
    trustchain_url:
        TrustChain service address. Default ``https://localhost:8443``.
    catalog_url:
        Catalog service address. Default ``https://localhost:8443``.
    ngauge_url:
        NGauge service address. Default ``https://localhost:8443``.
    async_mode:
        When ``True``, use httpx-based async transport. All API
        methods become coroutines.
    timeout:
        Request timeout in seconds.
    session_token:
        Phase K.2 — base64-encoded ``CapabilityToken`` issued by the
        daemon's ``auth.create_session`` IPC. When set, every request
        carries the ``X-HyperMesh-Capability`` header. Required when
        the daemon is configured for token enforcement; ignored by
        alpha-default inert daemons.
    """

    def __init__(
        self,
        base_url: str = _DEFAULT_BASE_URL,
        *,
        caesar_url: str = _DEFAULT_CAESAR_URL,
        trustchain_url: str = _DEFAULT_TRUSTCHAIN_URL,
        catalog_url: str = _DEFAULT_CATALOG_URL,
        ngauge_url: str = _DEFAULT_NGAUGE_URL,
        async_mode: bool = False,
        timeout: float = 30.0,
        session_token: str | None = None,
    ) -> None:
        self._base_url = base_url
        self._async_mode = async_mode
        self._session_token = session_token

        if async_mode:
            transport = AsyncTransport(
                base_url, timeout=timeout, capability_token=session_token
            )
            caesar_transport = AsyncTransport(
                caesar_url, timeout=timeout, capability_token=session_token
            )
            trustchain_transport = AsyncTransport(
                trustchain_url, timeout=timeout, capability_token=session_token
            )
            catalog_transport = AsyncTransport(
                catalog_url, timeout=timeout, capability_token=session_token
            )
            ngauge_transport = AsyncTransport(
                ngauge_url, timeout=timeout, capability_token=session_token
            )
            self._async_transport = transport
            self._async_service_transports = [
                caesar_transport,
                trustchain_transport,
                catalog_transport,
                ngauge_transport,
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
            from .api.ngauge import AsyncNGaugeApi
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
            self.ngauge = AsyncNGaugeApi(ngauge_transport)
            self.catalog = AsyncCatalogApi(catalog_transport)
        else:
            transport = SyncTransport(  # type: ignore[assignment]
                base_url, timeout=timeout, capability_token=session_token
            )
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
            from .api.ngauge import NGaugeApi
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
                SyncTransport(
                    caesar_url, timeout=timeout, capability_token=session_token
                )
            )
            self.trustchain = TrustChainApi(
                SyncTransport(
                    trustchain_url, timeout=timeout, capability_token=session_token
                )
            )
            self.ngauge = NGaugeApi(
                SyncTransport(
                    ngauge_url, timeout=timeout, capability_token=session_token
                )
            )
            self.catalog = CatalogApi(
                SyncTransport(
                    catalog_url, timeout=timeout, capability_token=session_token
                )
            )

        # Track every transport so set_capability_token can rotate the
        # token across the whole client in one call.
        self._all_transports: list[Any] = []
        for service_attr in (
            "node",
            "blockchain",
            "dns",
            "network",
            "topology",
            "asset",
            "dashboard",
            "config",
            "domain",
            "caesar",
            "trustchain",
            "ngauge",
            "catalog",
        ):
            api = getattr(self, service_attr, None)
            if api is None:
                continue
            t = getattr(api, "_transport", None)
            if t is not None and t not in self._all_transports:
                self._all_transports.append(t)

    def set_capability_token(self, token: str | None) -> None:
        """Phase K.2 — rotate the capability token on every transport.

        Pass ``None`` to clear the token (e.g. after revocation).
        """
        self._session_token = token
        for t in self._all_transports:
            if hasattr(t, "set_capability_token"):
                t.set_capability_token(token)

    def get_capability_token(self) -> str | None:
        """Phase K.2 — currently-installed token (or None)."""
        return self._session_token

    def auth_create_session(
        self,
        device_pubkey_hex: str,
        requested_capabilities: list[str],
        ttl_secs: int = 3600,
    ) -> Any:
        """Phase K.2 — issue a capability token via the daemon.

        Returns the raw JSON payload from ``auth.create_session``. The
        caller typically passes the returned token bytes (base64
        encoded) into :meth:`set_capability_token`.

        Sync clients only — async clients should call the underlying
        transport directly.
        """
        if self._async_mode:
            raise NotImplementedError(
                "auth_create_session is sync-only; use the underlying "
                "AsyncTransport.post('/api/v1/auth/create_session', ...)"
            )
        sync_t = self._all_transports[0]
        return sync_t.post(
            "/api/v1/auth/create_session",
            {
                "device_pubkey": device_pubkey_hex,
                "requested_capabilities": requested_capabilities,
                "ttl_secs": ttl_secs,
            },
        )

    def auth_list_sessions(self) -> Any:
        """Phase K.2 — list active sessions known to the daemon."""
        if self._async_mode:
            raise NotImplementedError("auth_list_sessions is sync-only")
        return self._all_transports[0].get("/api/v1/auth/list_sessions")

    def auth_revoke_session(self, session_id: str) -> Any:
        """Phase K.2 — revoke a session by id."""
        if self._async_mode:
            raise NotImplementedError("auth_revoke_session is sync-only")
        return self._all_transports[0].post(
            "/api/v1/auth/revoke_session",
            {"session_id": session_id},
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
    "HyperMeshFFI",
    "HyperMeshError",
    "FFIError",
    "LibraryNotFoundError",
    "ApiError",
    "ConnectionError",
    "NotFoundError",
    "CAPABILITY_TOKEN_HEADER",
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
    "NGaugeCapacityMetrics",
    "NGaugeTrafficMetrics",
    "NGaugeListingList",
    "NGaugeNodeMetrics",
    "NGaugeLeaseList",
    "CatalogPackageInfo",
    "CatalogPackageList",
    "CatalogSearchResults",
    "CatalogRegistryStats",
]
