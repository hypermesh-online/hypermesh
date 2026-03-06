using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Node configuration endpoints.
/// </summary>
public sealed class ConfigApi
{
    private readonly HttpApiClient _client;

    internal ConfigApi(HttpApiClient client) => _client = client;

    /// <summary>Get the full node configuration.</summary>
    public Task<ConfigData> ShowAsync(CancellationToken ct = default)
        => _client.GetAsync<ConfigData>("/api/v1/config/show", ct);

    /// <summary>Get a single configuration value by key.</summary>
    public Task<ConfigValue> GetAsync(string key, CancellationToken ct = default)
        => _client.GetAsync<ConfigValue>($"/api/v1/config/get/{Uri.EscapeDataString(key)}", ct);
}
