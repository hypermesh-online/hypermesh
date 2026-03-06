using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Asset management endpoints.
/// </summary>
public sealed class AssetApi
{
    private readonly HttpApiClient _client;

    internal AssetApi(HttpApiClient client) => _client = client;

    /// <summary>List all registered assets.</summary>
    public Task<AssetList> ListAsync(CancellationToken ct = default)
        => _client.GetAsync<AssetList>("/api/v1/asset/list", ct);
}
