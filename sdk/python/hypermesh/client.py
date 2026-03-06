"""HTTP client for the HyperMesh node API.

Provides both sync (urllib) and async (httpx) transports.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Any

from .errors import ApiError, ConnectionError, HyperMeshError, NotFoundError

_TIMEOUT_SECS = 30


class SyncTransport:
    """Synchronous HTTP transport using urllib (zero dependencies)."""

    def __init__(self, base_url: str, timeout: float = _TIMEOUT_SECS) -> None:
        self._base_url = base_url.rstrip("/")
        self._timeout = timeout

    def get(self, path: str) -> Any:
        return self._request("GET", path)

    def post(self, path: str, body: dict[str, Any] | None = None) -> Any:
        return self._request("POST", path, body)

    def _request(
        self,
        method: str,
        path: str,
        body: dict[str, Any] | None = None,
    ) -> Any:
        url = f"{self._base_url}{path}"
        data = json.dumps(body).encode() if body is not None else None
        headers = {"Content-Type": "application/json"} if data else {}

        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=self._timeout) as resp:
                raw = resp.read().decode()
                if not raw:
                    return {}
                return json.loads(raw)
        except urllib.error.HTTPError as exc:
            status = exc.code
            try:
                detail = json.loads(exc.read().decode())
                msg = detail.get("error", str(detail))
            except Exception:
                msg = exc.reason or str(exc)
            if status == 404:
                raise NotFoundError(msg) from exc
            raise ApiError(msg, status_code=status) from exc
        except urllib.error.URLError as exc:
            raise ConnectionError(
                f"Cannot connect to {self._base_url}: {exc.reason}"
            ) from exc
        except Exception as exc:
            raise HyperMeshError(str(exc)) from exc


class AsyncTransport:
    """Async HTTP transport using httpx."""

    def __init__(self, base_url: str, timeout: float = _TIMEOUT_SECS) -> None:
        self._base_url = base_url.rstrip("/")
        self._timeout = timeout
        self._client: Any = None

    async def _ensure_client(self) -> Any:
        if self._client is None:
            try:
                import httpx
            except ImportError as exc:
                raise ImportError(
                    "httpx is required for async mode. "
                    "Install it with: pip install hypermesh-sdk[async]"
                ) from exc
            self._client = httpx.AsyncClient(
                base_url=self._base_url,
                timeout=self._timeout,
            )
        return self._client

    async def get(self, path: str) -> Any:
        return await self._request("GET", path)

    async def post(self, path: str, body: dict[str, Any] | None = None) -> Any:
        return await self._request("POST", path, body)

    async def _request(
        self,
        method: str,
        path: str,
        body: dict[str, Any] | None = None,
    ) -> Any:
        try:
            import httpx
        except ImportError as exc:
            raise ImportError(
                "httpx is required for async mode. "
                "Install it with: pip install hypermesh-sdk[async]"
            ) from exc

        client = await self._ensure_client()
        try:
            resp = await client.request(method, path, json=body)
            if resp.status_code == 404:
                msg = "Resource not found"
                try:
                    detail = resp.json()
                    msg = detail.get("error", msg)
                except Exception:
                    pass
                raise NotFoundError(msg)
            resp.raise_for_status()
            if not resp.text:
                return {}
            return resp.json()
        except httpx.ConnectError as exc:
            raise ConnectionError(
                f"Cannot connect to {self._base_url}: {exc}"
            ) from exc
        except (NotFoundError, ApiError):
            raise
        except httpx.HTTPStatusError as exc:
            raise ApiError(
                str(exc), status_code=exc.response.status_code
            ) from exc
        except Exception as exc:
            if isinstance(exc, HyperMeshError):
                raise
            raise HyperMeshError(str(exc)) from exc

    async def close(self) -> None:
        if self._client is not None:
            await self._client.aclose()
            self._client = None
