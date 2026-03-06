"""DNS API: list, resolve, register."""

from __future__ import annotations

from typing import Any

from ..types import DnsList, DnsRecord

_PREFIX = "/api/v1/dns"


class DnsApi:
    """Wraps /api/v1/dns/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def list(self) -> DnsList:
        data = self._t.get(f"{_PREFIX}/list")
        return _parse_dns_list(data)

    def resolve(self, name: str) -> DnsRecord:
        data = self._t.get(f"{_PREFIX}/resolve/{name}")
        return DnsRecord(
            name=data.get("name", name),
            address=data.get("address", ""),
        )

    def register(self, name: str, address: str) -> dict[str, Any]:
        return self._t.post(
            f"{_PREFIX}/register",
            {"name": name, "address": address},
        )


class AsyncDnsApi:
    """Async variant of DnsApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def list(self) -> DnsList:
        data = await self._t.get(f"{_PREFIX}/list")
        return _parse_dns_list(data)

    async def resolve(self, name: str) -> DnsRecord:
        data = await self._t.get(f"{_PREFIX}/resolve/{name}")
        return DnsRecord(
            name=data.get("name", name),
            address=data.get("address", ""),
        )

    async def register(self, name: str, address: str) -> dict[str, Any]:
        return await self._t.post(
            f"{_PREFIX}/register",
            {"name": name, "address": address},
        )


def _parse_dns_list(data: dict[str, Any]) -> DnsList:
    records = [
        DnsRecord(name=r.get("name", ""), address=r.get("address", ""))
        for r in data.get("records", [])
    ]
    return DnsList(count=data.get("count", len(records)), records=records)
