"""Network API: peer listing."""

from __future__ import annotations

from typing import Any

from ..types import Peer, PeerList

_PREFIX = "/api/v1/network"


class NetworkApi:
    """Wraps /api/v1/network/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def peers(self) -> PeerList:
        data = self._t.get(f"{_PREFIX}/peers")
        return _parse_peer_list(data)


class AsyncNetworkApi:
    """Async variant of NetworkApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def peers(self) -> PeerList:
        data = await self._t.get(f"{_PREFIX}/peers")
        return _parse_peer_list(data)


def _parse_peer_list(data: dict[str, Any]) -> PeerList:
    peers = [
        Peer(
            node_id=p.get("node_id", ""),
            address=p.get("address", ""),
            connected_at=p.get("connected_at", ""),
        )
        for p in data.get("peers", [])
    ]
    return PeerList(count=data.get("count", len(peers)), peers=peers)
