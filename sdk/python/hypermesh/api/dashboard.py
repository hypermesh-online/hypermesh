"""Dashboard API: listing and info."""

from __future__ import annotations

from typing import Any

from ..types import Dashboard, DashboardInfo, DashboardList

_PREFIX = "/api/v1/dashboard"


class DashboardApi:
    """Wraps /api/v1/dashboard/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def list(self) -> DashboardList:
        data = self._t.get(f"{_PREFIX}/list")
        return _parse_dashboard_list(data)

    def info(self) -> DashboardInfo:
        data = self._t.get(f"{_PREFIX}/info")
        return _parse_dashboard_info(data)


class AsyncDashboardApi:
    """Async variant of DashboardApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def list(self) -> DashboardList:
        data = await self._t.get(f"{_PREFIX}/list")
        return _parse_dashboard_list(data)

    async def info(self) -> DashboardInfo:
        data = await self._t.get(f"{_PREFIX}/info")
        return _parse_dashboard_info(data)


def _parse_dashboard_list(data: dict[str, Any]) -> DashboardList:
    dashboards = [
        Dashboard(
            name=d.get("name", ""),
            scope=d.get("scope", ""),
            url=d.get("url", ""),
        )
        for d in data.get("dashboards", [])
    ]
    return DashboardList(
        count=data.get("count", len(dashboards)),
        dashboards=dashboards,
    )


def _parse_dashboard_info(data: dict[str, Any]) -> DashboardInfo:
    return DashboardInfo(
        name=data.get("name", ""),
        version=data.get("version", ""),
        scope=data.get("scope", ""),
        metadata=data.get("metadata", {}),
    )
