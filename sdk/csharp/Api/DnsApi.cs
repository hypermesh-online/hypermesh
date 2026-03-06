using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// DNS record management endpoints.
/// </summary>
public sealed class DnsApi
{
    private readonly HttpApiClient _client;

    internal DnsApi(HttpApiClient client) => _client = client;

    /// <summary>List all DNS records.</summary>
    public Task<DnsList> ListAsync(CancellationToken ct = default)
        => _client.GetAsync<DnsList>("/api/v1/dns/list", ct);

    /// <summary>Resolve a DNS name to an address.</summary>
    public Task<DnsRecord> ResolveAsync(string name, CancellationToken ct = default)
        => _client.GetAsync<DnsRecord>($"/api/v1/dns/resolve/{Uri.EscapeDataString(name)}", ct);

    /// <summary>Register a new DNS record.</summary>
    public Task<DnsRegisterResponse> RegisterAsync(string name, string address, CancellationToken ct = default)
        => _client.PostAsync<DnsRegisterResponse>(
            "/api/v1/dns/register",
            new DnsRegisterRequest { Name = name, Address = address },
            ct);
}
