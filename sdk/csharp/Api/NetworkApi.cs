using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Network peer management endpoints.
/// </summary>
public sealed class NetworkApi
{
    private readonly HttpApiClient _client;

    internal NetworkApi(HttpApiClient client) => _client = client;

    /// <summary>List connected peers.</summary>
    public Task<PeerList> PeersAsync(CancellationToken ct = default)
        => _client.GetAsync<PeerList>("/api/v1/network/peers", ct);
}
