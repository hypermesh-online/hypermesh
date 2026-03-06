"""Engauge API: capacity, traffic, marketplace, node metrics, leases."""

from __future__ import annotations

from typing import Any

from ..types import (
    EngaugeCapacityMetrics,
    EngaugeLeaseList,
    EngaugeListingList,
    EngaugeNodeMetrics,
    EngaugeTrafficMetrics,
)

_PREFIX = "/api/v1/engauge"


class EngaugeApi:
    """Wraps /api/v1/engauge/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def capacity(self) -> EngaugeCapacityMetrics:
        data = self._t.get(f"{_PREFIX}/capacity")
        return _parse_capacity(data)

    def traffic(self) -> EngaugeTrafficMetrics:
        data = self._t.get(f"{_PREFIX}/traffic")
        return EngaugeTrafficMetrics(
            organic_ratio=data.get("organic_ratio", 0.0),
            speculative_ratio=data.get("speculative_ratio", 0.0),
            total_requests=data.get("total_requests", 0),
        )

    def marketplace_listings(self) -> EngaugeListingList:
        data = self._t.get(f"{_PREFIX}/marketplace/listings")
        return _parse_listing_list(data)

    def node_metrics(self) -> EngaugeNodeMetrics:
        data = self._t.get(f"{_PREFIX}/node/metrics")
        return EngaugeNodeMetrics(
            activity_score=data.get("activity_score", 0.0),
            receipts=data.get("receipts", 0),
            bandwidth=data.get("bandwidth", 0.0),
        )

    def leases(self) -> EngaugeLeaseList:
        data = self._t.get(f"{_PREFIX}/leases")
        return _parse_lease_list(data)


class AsyncEngaugeApi:
    """Async variant of EngaugeApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def capacity(self) -> EngaugeCapacityMetrics:
        data = await self._t.get(f"{_PREFIX}/capacity")
        return _parse_capacity(data)

    async def traffic(self) -> EngaugeTrafficMetrics:
        data = await self._t.get(f"{_PREFIX}/traffic")
        return EngaugeTrafficMetrics(
            organic_ratio=data.get("organic_ratio", 0.0),
            speculative_ratio=data.get("speculative_ratio", 0.0),
            total_requests=data.get("total_requests", 0),
        )

    async def marketplace_listings(self) -> EngaugeListingList:
        data = await self._t.get(f"{_PREFIX}/marketplace/listings")
        return _parse_listing_list(data)

    async def node_metrics(self) -> EngaugeNodeMetrics:
        data = await self._t.get(f"{_PREFIX}/node/metrics")
        return EngaugeNodeMetrics(
            activity_score=data.get("activity_score", 0.0),
            receipts=data.get("receipts", 0),
            bandwidth=data.get("bandwidth", 0.0),
        )

    async def leases(self) -> EngaugeLeaseList:
        data = await self._t.get(f"{_PREFIX}/leases")
        return _parse_lease_list(data)


def _parse_capacity(data: dict[str, Any]) -> EngaugeCapacityMetrics:
    return EngaugeCapacityMetrics(
        bytes_served=data.get("bytes_served", 0),
        compute_delivered=data.get("compute_delivered", 0.0),
        storage=data.get("storage", 0),
        bandwidth=data.get("bandwidth", 0.0),
        uptime=data.get("uptime", 0.0),
    )


def _parse_listing_list(data: dict[str, Any]) -> EngaugeListingList:
    listings = data.get("listings", [])
    return EngaugeListingList(
        count=data.get("count", len(listings)),
        listings=listings,
    )


def _parse_lease_list(data: dict[str, Any]) -> EngaugeLeaseList:
    leases = data.get("leases", [])
    return EngaugeLeaseList(
        count=data.get("count", len(leases)),
        leases=leases,
    )
