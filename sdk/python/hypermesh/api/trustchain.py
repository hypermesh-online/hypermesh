"""TrustChain API: certificates, issuance, validation, revocation, DNS zones."""

from __future__ import annotations

from typing import Any

from ..types import (
    TrustChainCertificate,
    TrustChainCertificateList,
    TrustChainDnsZoneList,
    TrustChainRevokeResult,
    TrustChainValidationResult,
)

_PREFIX = "/api/v1/trustchain"


class TrustChainApi:
    """Wraps /api/v1/trustchain/* endpoints."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    def certificates(self) -> TrustChainCertificateList:
        data = self._t.get(f"{_PREFIX}/certificates")
        return _parse_certificate_list(data)

    def issue(self, subject: str, scope: str) -> TrustChainCertificate:
        data = self._t.post(
            f"{_PREFIX}/issue", {"subject": subject, "scope": scope}
        )
        return _parse_certificate(data)

    def validate(self, cert_pem: str) -> TrustChainValidationResult:
        data = self._t.post(f"{_PREFIX}/validate", {"cert_pem": cert_pem})
        return TrustChainValidationResult(
            valid=data.get("valid", False),
            errors=data.get("errors", []),
            chain_valid=data.get("chain_valid", False),
        )

    def revoke(self, cert_id: str) -> TrustChainRevokeResult:
        data = self._t.post(f"{_PREFIX}/revoke", {"cert_id": cert_id})
        return TrustChainRevokeResult(
            revoked=data.get("revoked", False),
            cert_id=data.get("cert_id", cert_id),
        )

    def dns_zones(self) -> TrustChainDnsZoneList:
        data = self._t.get(f"{_PREFIX}/dns/zones")
        return _parse_dns_zone_list(data)


class AsyncTrustChainApi:
    """Async variant of TrustChainApi."""

    def __init__(self, transport: Any) -> None:
        self._t = transport

    async def certificates(self) -> TrustChainCertificateList:
        data = await self._t.get(f"{_PREFIX}/certificates")
        return _parse_certificate_list(data)

    async def issue(self, subject: str, scope: str) -> TrustChainCertificate:
        data = await self._t.post(
            f"{_PREFIX}/issue", {"subject": subject, "scope": scope}
        )
        return _parse_certificate(data)

    async def validate(self, cert_pem: str) -> TrustChainValidationResult:
        data = await self._t.post(
            f"{_PREFIX}/validate", {"cert_pem": cert_pem}
        )
        return TrustChainValidationResult(
            valid=data.get("valid", False),
            errors=data.get("errors", []),
            chain_valid=data.get("chain_valid", False),
        )

    async def revoke(self, cert_id: str) -> TrustChainRevokeResult:
        data = await self._t.post(f"{_PREFIX}/revoke", {"cert_id": cert_id})
        return TrustChainRevokeResult(
            revoked=data.get("revoked", False),
            cert_id=data.get("cert_id", cert_id),
        )

    async def dns_zones(self) -> TrustChainDnsZoneList:
        data = await self._t.get(f"{_PREFIX}/dns/zones")
        return _parse_dns_zone_list(data)


def _parse_certificate(data: dict[str, Any]) -> TrustChainCertificate:
    return TrustChainCertificate(
        id=data.get("id", ""),
        subject=data.get("subject", ""),
        scope=data.get("scope", ""),
        valid_from=data.get("valid_from", ""),
        valid_to=data.get("valid_to", ""),
        pem=data.get("pem", ""),
    )


def _parse_certificate_list(
    data: dict[str, Any],
) -> TrustChainCertificateList:
    certs = [_parse_certificate(c) for c in data.get("certificates", [])]
    return TrustChainCertificateList(
        count=data.get("count", len(certs)),
        certificates=certs,
    )


def _parse_dns_zone_list(data: dict[str, Any]) -> TrustChainDnsZoneList:
    zones = data.get("zones", [])
    return TrustChainDnsZoneList(
        count=data.get("count", len(zones)),
        zones=zones,
    )
