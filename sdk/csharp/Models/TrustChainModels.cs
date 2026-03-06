using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record TrustChainCertificate
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("subject")]
    public string Subject { get; init; } = string.Empty;

    [JsonPropertyName("scope")]
    public string Scope { get; init; } = string.Empty;

    [JsonPropertyName("valid_from")]
    public string ValidFrom { get; init; } = string.Empty;

    [JsonPropertyName("valid_to")]
    public string ValidTo { get; init; } = string.Empty;

    [JsonPropertyName("pem")]
    public string Pem { get; init; } = string.Empty;
}

public sealed record TrustChainCertificateList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("certificates")]
    public List<TrustChainCertificate> Certificates { get; init; } = [];
}

public sealed record TrustChainIssueRequest
{
    [JsonPropertyName("subject")]
    public string Subject { get; init; } = string.Empty;

    [JsonPropertyName("scope")]
    public string Scope { get; init; } = string.Empty;
}

public sealed record TrustChainValidateRequest
{
    [JsonPropertyName("cert_pem")]
    public string CertPem { get; init; } = string.Empty;
}

public sealed record TrustChainValidationResult
{
    [JsonPropertyName("valid")]
    public bool Valid { get; init; }

    [JsonPropertyName("errors")]
    public List<string> Errors { get; init; } = [];

    [JsonPropertyName("chain_valid")]
    public bool ChainValid { get; init; }
}

public sealed record TrustChainRevokeRequest
{
    [JsonPropertyName("cert_id")]
    public string CertId { get; init; } = string.Empty;
}

public sealed record TrustChainRevokeResult
{
    [JsonPropertyName("revoked")]
    public bool Revoked { get; init; }

    [JsonPropertyName("cert_id")]
    public string CertId { get; init; } = string.Empty;
}

public sealed record TrustChainDnsZone
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("records")]
    public int Records { get; init; }
}

public sealed record TrustChainDnsZoneList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("zones")]
    public List<TrustChainDnsZone> Zones { get; init; } = [];
}
