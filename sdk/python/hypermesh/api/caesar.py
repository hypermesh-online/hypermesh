"""Caesar EVP API: wallet, balance, transactions, rewards, routing, governor."""

from __future__ import annotations

from typing import Any

from ..types import (
    CaesarBalance,
    CaesarGovernorParams,
    CaesarRewardInfo,
    CaesarRouteResult,
    CaesarTransactionList,
    CaesarWalletInfo,
)

_PREFIX = "/api/v1/caesar"


class CaesarApi:
    """Wraps /api/v1/caesar/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def wallet(self) -> CaesarWalletInfo:
        data = self._t.get(f"{_PREFIX}/wallet")
        return _parse_wallet(data)

    def balance(self) -> CaesarBalance:
        data = self._t.get(f"{_PREFIX}/balance")
        return CaesarBalance(
            gold_grams=data.get("gold_grams", 0.0),
            usd_equivalent=data.get("usd_equivalent", 0.0),
            tier=data.get("tier", ""),
        )

    def transactions(self, limit: int | None = None) -> CaesarTransactionList:
        path = f"{_PREFIX}/transactions"
        if limit is not None:
            path = f"{path}?limit={limit}"
        data = self._t.get(path)
        return _parse_transaction_list(data)

    def rewards(self) -> CaesarRewardInfo:
        data = self._t.get(f"{_PREFIX}/rewards")
        return CaesarRewardInfo(
            total_earned=data.get("total_earned", 0.0),
            pending=data.get("pending", 0.0),
            tier_multiplier=data.get("tier_multiplier", 0.0),
        )

    def route_packet(
        self, destination: str, amount_grams: float
    ) -> CaesarRouteResult:
        data = self._t.post(
            f"{_PREFIX}/route",
            {"destination": destination, "amount_grams": amount_grams},
        )
        return CaesarRouteResult(
            packet_id=data.get("packet_id", ""),
            status=data.get("status", ""),
            fee=data.get("fee", 0.0),
        )

    def governor_params(self) -> CaesarGovernorParams:
        data = self._t.get(f"{_PREFIX}/governor/params")
        return CaesarGovernorParams(
            velocity=data.get("velocity", 0.0),
            fee_rate=data.get("fee_rate", 0.0),
            demurrage_rate=data.get("demurrage_rate", 0.0),
        )


class AsyncCaesarApi:
    """Async variant of CaesarApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def wallet(self) -> CaesarWalletInfo:
        data = await self._t.get(f"{_PREFIX}/wallet")
        return _parse_wallet(data)

    async def balance(self) -> CaesarBalance:
        data = await self._t.get(f"{_PREFIX}/balance")
        return CaesarBalance(
            gold_grams=data.get("gold_grams", 0.0),
            usd_equivalent=data.get("usd_equivalent", 0.0),
            tier=data.get("tier", ""),
        )

    async def transactions(
        self, limit: int | None = None
    ) -> CaesarTransactionList:
        path = f"{_PREFIX}/transactions"
        if limit is not None:
            path = f"{path}?limit={limit}"
        data = await self._t.get(path)
        return _parse_transaction_list(data)

    async def rewards(self) -> CaesarRewardInfo:
        data = await self._t.get(f"{_PREFIX}/rewards")
        return CaesarRewardInfo(
            total_earned=data.get("total_earned", 0.0),
            pending=data.get("pending", 0.0),
            tier_multiplier=data.get("tier_multiplier", 0.0),
        )

    async def route_packet(
        self, destination: str, amount_grams: float
    ) -> CaesarRouteResult:
        data = await self._t.post(
            f"{_PREFIX}/route",
            {"destination": destination, "amount_grams": amount_grams},
        )
        return CaesarRouteResult(
            packet_id=data.get("packet_id", ""),
            status=data.get("status", ""),
            fee=data.get("fee", 0.0),
        )

    async def governor_params(self) -> CaesarGovernorParams:
        data = await self._t.get(f"{_PREFIX}/governor/params")
        return CaesarGovernorParams(
            velocity=data.get("velocity", 0.0),
            fee_rate=data.get("fee_rate", 0.0),
            demurrage_rate=data.get("demurrage_rate", 0.0),
        )


def _parse_wallet(data: dict[str, Any]) -> CaesarWalletInfo:
    return CaesarWalletInfo(
        balance_grams=data.get("balance_grams", 0.0),
        balance_usd=data.get("balance_usd", 0.0),
        tier=data.get("tier", ""),
        node_id=data.get("node_id", ""),
    )


def _parse_transaction_list(data: dict[str, Any]) -> CaesarTransactionList:
    transactions = data.get("transactions", [])
    return CaesarTransactionList(
        count=data.get("count", len(transactions)),
        transactions=transactions,
    )
