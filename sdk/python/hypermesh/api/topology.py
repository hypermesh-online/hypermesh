"""Topology API: info and neighbors."""

from __future__ import annotations

from typing import Any

from ..types import Neighbor, Neighbors, TopologyInfo

_PREFIX = "/api/v1/topology"


class TopologyApi:
    """Wraps /api/v1/topology/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def info(self) -> TopologyInfo:
        data = self._t.get(f"{_PREFIX}/info")
        return TopologyInfo(
            coordinate=data.get("coordinate", {}),
            node_id=data.get("node_id", ""),
        )

    def neighbors(self) -> Neighbors:
        data = self._t.get(f"{_PREFIX}/neighbors")
        return _parse_neighbors(data)


class AsyncTopologyApi:
    """Async variant of TopologyApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def info(self) -> TopologyInfo:
        data = await self._t.get(f"{_PREFIX}/info")
        return TopologyInfo(
            coordinate=data.get("coordinate", {}),
            node_id=data.get("node_id", ""),
        )

    async def neighbors(self) -> Neighbors:
        data = await self._t.get(f"{_PREFIX}/neighbors")
        return _parse_neighbors(data)


def _parse_neighbors(data: dict[str, Any]) -> Neighbors:
    neighbors = [
        Neighbor(
            node_id=n.get("node_id", ""),
            coordinate=n.get("coordinate", {}),
            distance=n.get("distance", 0.0),
        )
        for n in data.get("neighbors", [])
    ]
    return Neighbors(
        center=data.get("center", {}),
        count=data.get("count", len(neighbors)),
        neighbors=neighbors,
        radius=data.get("radius", 0.0),
    )
