using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// TrustChain endpoints (certificates, issuance, validation, revocation, DNS zones).
/// </summary>
public sealed class TrustChainApi
{
    private readonly HttpApiClient _client;

    internal TrustChainApi(HttpApiClient client) => _client = client;

    /// <summary>List all certificates.</summary>
    public Task<TrustChainCertificateList> CertificatesAsync(CancellationToken ct = default)
        => _client.GetAsync<TrustChainCertificateList>("/api/v1/trustchain/certificates", ct);

    /// <summary>Issue a new certificate.</summary>
    public Task<TrustChainCertificate> IssueAsync(string subject, string scope, CancellationToken ct = default)
        => _client.PostAsync<TrustChainCertificate>(
            "/api/v1/trustchain/issue",
            new TrustChainIssueRequest { Subject = subject, Scope = scope },
            ct);

    /// <summary>Validate a certificate.</summary>
    public Task<TrustChainValidationResult> ValidateAsync(string certPem, CancellationToken ct = default)
        => _client.PostAsync<TrustChainValidationResult>(
            "/api/v1/trustchain/validate",
            new TrustChainValidateRequest { CertPem = certPem },
            ct);

    /// <summary>Revoke a certificate.</summary>
    public Task<TrustChainRevokeResult> RevokeAsync(string certId, CancellationToken ct = default)
        => _client.PostAsync<TrustChainRevokeResult>(
            "/api/v1/trustchain/revoke",
            new TrustChainRevokeRequest { CertId = certId },
            ct);

    /// <summary>List DNS zones.</summary>
    public Task<TrustChainDnsZoneList> DnsZonesAsync(CancellationToken ct = default)
        => _client.GetAsync<TrustChainDnsZoneList>("/api/v1/trustchain/dns/zones", ct);
}
