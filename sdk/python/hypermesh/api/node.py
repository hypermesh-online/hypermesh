"""Node API: status and health checks."""

from __future__ import annotations

from typing import Any

from ..types import NodeStatus

_PREFIX = "/api/v1"


class NodeApi:
    """Wraps /api/v1/status and /api/v1/ping."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def status(self) -> NodeStatus:
        data = self._t.get(f"{_PREFIX}/status")
        return NodeStatus(
            chain_height=data.get("chain_height", 0),
            coordinate=data.get("coordinate", {}),
            node_id=data.get("node_id", ""),
            peers=data.get("peers", 0),
            privacy_mode=data.get("privacy_mode", ""),
            uptime_secs=data.get("uptime_secs", 0.0),
        )

    def ping(self) -> bool:
        data = self._t.get(f"{_PREFIX}/ping")
        return bool(data.get("pong", False))


class AsyncNodeApi:
    """Async variant of NodeApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def status(self) -> NodeStatus:
        data = await self._t.get(f"{_PREFIX}/status")
        return NodeStatus(
            chain_height=data.get("chain_height", 0),
            coordinate=data.get("coordinate", {}),
            node_id=data.get("node_id", ""),
            peers=data.get("peers", 0),
            privacy_mode=data.get("privacy_mode", ""),
            uptime_secs=data.get("uptime_secs", 0.0),
        )

    async def ping(self) -> bool:
        data = await self._t.get(f"{_PREFIX}/ping")
        return bool(data.get("pong", False))
