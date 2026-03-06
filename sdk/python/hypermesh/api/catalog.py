"""Catalog API: browse, search, package info, registry stats."""

from __future__ import annotations

from typing import Any
from urllib.parse import quote

from ..types import (
    CatalogPackageInfo,
    CatalogPackageList,
    CatalogRegistryStats,
    CatalogSearchResults,
)

_PREFIX = "/api/v1/catalog"


class CatalogApi:
    """Wraps /api/v1/catalog/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def browse(
        self, query: str | None = None, page: int | None = None
    ) -> CatalogPackageList:
        params: list[str] = []
        if query is not None:
            params.append(f"query={quote(query)}")
        if page is not None:
            params.append(f"page={page}")
        qs = f"?{'&'.join(params)}" if params else ""
        data = self._t.get(f"{_PREFIX}/browse{qs}")
        return _parse_package_list(data)

    def search(self, query: str) -> CatalogSearchResults:
        data = self._t.get(f"{_PREFIX}/search?query={quote(query)}")
        return _parse_search_results(data)

    def package_info(self, name: str) -> CatalogPackageInfo:
        data = self._t.get(f"{_PREFIX}/package/{quote(name)}")
        return _parse_package_info(data)

    def registry_stats(self) -> CatalogRegistryStats:
        data = self._t.get(f"{_PREFIX}/registry/stats")
        return CatalogRegistryStats(
            package_count=data.get("package_count", 0),
            publisher_count=data.get("publisher_count", 0),
            total_downloads=data.get("total_downloads", 0),
        )


class AsyncCatalogApi:
    """Async variant of CatalogApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def browse(
        self, query: str | None = None, page: int | None = None
    ) -> CatalogPackageList:
        params: list[str] = []
        if query is not None:
            params.append(f"query={quote(query)}")
        if page is not None:
            params.append(f"page={page}")
        qs = f"?{'&'.join(params)}" if params else ""
        data = await self._t.get(f"{_PREFIX}/browse{qs}")
        return _parse_package_list(data)

    async def search(self, query: str) -> CatalogSearchResults:
        data = await self._t.get(f"{_PREFIX}/search?query={quote(query)}")
        return _parse_search_results(data)

    async def package_info(self, name: str) -> CatalogPackageInfo:
        data = await self._t.get(f"{_PREFIX}/package/{quote(name)}")
        return _parse_package_info(data)

    async def registry_stats(self) -> CatalogRegistryStats:
        data = await self._t.get(f"{_PREFIX}/registry/stats")
        return CatalogRegistryStats(
            package_count=data.get("package_count", 0),
            publisher_count=data.get("publisher_count", 0),
            total_downloads=data.get("total_downloads", 0),
        )


def _parse_package_list(data: dict[str, Any]) -> CatalogPackageList:
    packages = [
        _parse_package_info(p) for p in data.get("packages", [])
    ]
    return CatalogPackageList(
        count=data.get("count", len(packages)),
        packages=packages,
    )


def _parse_search_results(data: dict[str, Any]) -> CatalogSearchResults:
    results = data.get("results", [])
    return CatalogSearchResults(
        count=data.get("count", len(results)),
        results=results,
    )


def _parse_package_info(data: dict[str, Any]) -> CatalogPackageInfo:
    return CatalogPackageInfo(
        name=data.get("name", ""),
        version=data.get("version", ""),
        description=data.get("description", ""),
        author=data.get("author", ""),
        downloads=data.get("downloads", 0),
    )
