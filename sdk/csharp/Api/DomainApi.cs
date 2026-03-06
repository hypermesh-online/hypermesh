using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Domain registration and management endpoints.
/// </summary>
public sealed class DomainApi
{
    private readonly HttpApiClient _client;

    internal DomainApi(HttpApiClient client) => _client = client;

    /// <summary>List registered domains.</summary>
    public Task<DomainList> ListAsync(CancellationToken ct = default)
        => _client.GetAsync<DomainList>("/api/v1/domain/list", ct);

    /// <summary>Register a new domain.</summary>
    public Task<DomainRegisterResponse> RegisterAsync(string name, string privacy, CancellationToken ct = default)
        => _client.PostAsync<DomainRegisterResponse>(
            "/api/v1/domain/register",
            new DomainRegisterRequest { Name = name, Privacy = privacy },
            ct);

    /// <summary>Join an existing domain.</summary>
    public Task<DomainJoinResponse> JoinAsync(string name, string? token = null, CancellationToken ct = default)
        => _client.PostAsync<DomainJoinResponse>(
            "/api/v1/domain/join",
            new DomainJoinRequest { Name = name, Token = token },
            ct);
}
