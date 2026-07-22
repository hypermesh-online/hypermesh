"""NGauge API: capacity, traffic, marketplace, node metrics, leases."""

from __future__ import annotations

from typing import Any

from ..types import (
    NGaugeCapacityMetrics,
    NGaugeLeaseList,
    NGaugeListingList,
    NGaugeNodeMetrics,
    NGaugeTrafficMetrics,
)

_PREFIX = "/api/v1/ngauge"


class NGaugeApi:
    """Wraps /api/v1/ngauge/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def capacity(self) -> NGaugeCapacityMetrics:
        data = self._t.get(f"{_PREFIX}/capacity")
        return _parse_capacity(data)

    def traffic(self) -> NGaugeTrafficMetrics:
        data = self._t.get(f"{_PREFIX}/traffic")
        return NGaugeTrafficMetrics(
            organic_ratio=data.get("organic_ratio", 0.0),
            speculative_ratio=data.get("speculative_ratio", 0.0),
            total_requests=data.get("total_requests", 0),
        )

    def marketplace_listings(self) -> NGaugeListingList:
        data = self._t.get(f"{_PREFIX}/marketplace/listings")
        return _parse_listing_list(data)

    def node_metrics(self) -> NGaugeNodeMetrics:
        data = self._t.get(f"{_PREFIX}/node/metrics")
        return NGaugeNodeMetrics(
            activity_score=data.get("activity_score", 0.0),
            receipts=data.get("receipts", 0),
            bandwidth=data.get("bandwidth", 0.0),
        )

    def leases(self) -> NGaugeLeaseList:
        data = self._t.get(f"{_PREFIX}/leases")
        return _parse_lease_list(data)


class AsyncNGaugeApi:
    """Async variant of NGaugeApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def capacity(self) -> NGaugeCapacityMetrics:
        data = await self._t.get(f"{_PREFIX}/capacity")
        return _parse_capacity(data)

    async def traffic(self) -> NGaugeTrafficMetrics:
        data = await self._t.get(f"{_PREFIX}/traffic")
        return NGaugeTrafficMetrics(
            organic_ratio=data.get("organic_ratio", 0.0),
            speculative_ratio=data.get("speculative_ratio", 0.0),
            total_requests=data.get("total_requests", 0),
        )

    async def marketplace_listings(self) -> NGaugeListingList:
        data = await self._t.get(f"{_PREFIX}/marketplace/listings")
        return _parse_listing_list(data)

    async def node_metrics(self) -> NGaugeNodeMetrics:
        data = await self._t.get(f"{_PREFIX}/node/metrics")
        return NGaugeNodeMetrics(
            activity_score=data.get("activity_score", 0.0),
            receipts=data.get("receipts", 0),
            bandwidth=data.get("bandwidth", 0.0),
        )

    async def leases(self) -> NGaugeLeaseList:
        data = await self._t.get(f"{_PREFIX}/leases")
        return _parse_lease_list(data)


def _parse_capacity(data: dict[str, Any]) -> NGaugeCapacityMetrics:
    return NGaugeCapacityMetrics(
        bytes_served=data.get("bytes_served", 0),
        compute_delivered=data.get("compute_delivered", 0.0),
        storage=data.get("storage", 0),
        bandwidth=data.get("bandwidth", 0.0),
        uptime=data.get("uptime", 0.0),
    )


def _parse_listing_list(data: dict[str, Any]) -> NGaugeListingList:
    listings = data.get("listings", [])
    return NGaugeListingList(
        count=data.get("count", len(listings)),
        listings=listings,
    )


def _parse_lease_list(data: dict[str, Any]) -> NGaugeLeaseList:
    leases = data.get("leases", [])
    return NGaugeLeaseList(
        count=data.get("count", len(leases)),
        leases=leases,
    )
