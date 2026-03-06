"""Config API: show and get configuration."""

from __future__ import annotations

from typing import Any

_PREFIX = "/api/v1/config"


class ConfigApi:
    """Wraps /api/v1/config/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def show(self) -> dict[str, Any]:
        return self._t.get(f"{_PREFIX}/show")

    def get(self, key: str) -> Any:
        return self._t.get(f"{_PREFIX}/get/{key}")


class AsyncConfigApi:
    """Async variant of ConfigApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def show(self) -> dict[str, Any]:
        return await self._t.get(f"{_PREFIX}/show")

    async def get(self, key: str) -> Any:
        return await self._t.get(f"{_PREFIX}/get/{key}")
