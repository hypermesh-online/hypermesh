"""Asset API: listing registered assets."""

from __future__ import annotations

from typing import Any

from ..types import Asset, AssetList

_PREFIX = "/api/v1/asset"


class AssetApi:
    """Wraps /api/v1/asset/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def list(self) -> AssetList:
        data = self._t.get(f"{_PREFIX}/list")
        return _parse_asset_list(data)


class AsyncAssetApi:
    """Async variant of AssetApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def list(self) -> AssetList:
        data = await self._t.get(f"{_PREFIX}/list")
        return _parse_asset_list(data)


def _parse_asset_list(data: dict[str, Any]) -> AssetList:
    assets = [
        Asset(
            asset_id=a.get("asset_id", ""),
            asset_type=a.get("asset_type", ""),
            state=a.get("state", ""),
            metadata=a.get("metadata", {}),
        )
        for a in data.get("assets", [])
    ]
    return AssetList(count=data.get("count", len(assets)), assets=assets)
