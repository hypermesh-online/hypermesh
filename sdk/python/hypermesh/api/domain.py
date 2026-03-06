"""Domain API: list, register, join."""

from __future__ import annotations

from typing import Any

from ..types import Domain, DomainList

_PREFIX = "/api/v1/domain"


class DomainApi:
    """Wraps /api/v1/domain/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def list(self) -> DomainList:
        data = self._t.get(f"{_PREFIX}/list")
        return _parse_domain_list(data)

    def register(self, name: str, privacy: str) -> dict[str, Any]:
        return self._t.post(
            f"{_PREFIX}/register",
            {"name": name, "privacy": privacy},
        )

    def join(self, name: str, token: str | None = None) -> dict[str, Any]:
        body: dict[str, Any] = {"name": name}
        if token is not None:
            body["token"] = token
        return self._t.post(f"{_PREFIX}/join", body)


class AsyncDomainApi:
    """Async variant of DomainApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def list(self) -> DomainList:
        data = await self._t.get(f"{_PREFIX}/list")
        return _parse_domain_list(data)

    async def register(self, name: str, privacy: str) -> dict[str, Any]:
        return await self._t.post(
            f"{_PREFIX}/register",
            {"name": name, "privacy": privacy},
        )

    async def join(self, name: str, token: str | None = None) -> dict[str, Any]:
        body: dict[str, Any] = {"name": name}
        if token is not None:
            body["token"] = token
        return await self._t.post(f"{_PREFIX}/join", body)


def _parse_domain_list(data: dict[str, Any]) -> DomainList:
    domains = [
        Domain(
            name=d.get("name", ""),
            privacy=d.get("privacy", ""),
            owner=d.get("owner", ""),
        )
        for d in data.get("domains", [])
    ]
    return DomainList(count=data.get("count", len(domains)), domains=domains)
