"""HyperMesh SDK error types."""

from __future__ import annotations


class HyperMeshError(Exception):
    """Base error for all HyperMesh SDK operations."""

    def __init__(self, message: str, status_code: int | None = None) -> None:
        self.status_code = status_code
        super().__init__(message)


class ConnectionError(HyperMeshError):
    """Failed to connect to the HyperMesh node."""


class NotFoundError(HyperMeshError):
    """Requested resource was not found (HTTP 404)."""

    def __init__(self, message: str = "Resource not found") -> None:
        super().__init__(message, status_code=404)


class ApiError(HyperMeshError):
    """The node returned an error response."""
