"""Blockchain API: chain height, blocks, validation."""

from __future__ import annotations

from typing import Any

from ..types import Block, BlockchainHeight, ValidationResult

_PREFIX = "/api/v1/blockchain"


class BlockchainApi:
    """Wraps /api/v1/blockchain/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def height(self) -> BlockchainHeight:
        data = self._t.get(f"{_PREFIX}/height")
        return BlockchainHeight(height=data.get("height", 0))

    def block(self, index: int) -> Block:
        data = self._t.get(f"{_PREFIX}/block/{index}")
        return _parse_block(data)

    def validate(self) -> ValidationResult:
        data = self._t.get(f"{_PREFIX}/validate")
        return ValidationResult(
            valid=data.get("valid", False),
            errors=data.get("errors", []),
        )


class AsyncBlockchainApi:
    """Async variant of BlockchainApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def height(self) -> BlockchainHeight:
        data = await self._t.get(f"{_PREFIX}/height")
        return BlockchainHeight(height=data.get("height", 0))

    async def block(self, index: int) -> Block:
        data = await self._t.get(f"{_PREFIX}/block/{index}")
        return _parse_block(data)

    async def validate(self) -> ValidationResult:
        data = await self._t.get(f"{_PREFIX}/validate")
        return ValidationResult(
            valid=data.get("valid", False),
            errors=data.get("errors", []),
        )


def _parse_block(data: dict[str, Any]) -> Block:
    return Block(
        index=data.get("index", 0),
        timestamp=data.get("timestamp", ""),
        data=data.get("data", {}),
        hash=data.get("hash", ""),
        previous_hash=data.get("previous_hash", ""),
    )
