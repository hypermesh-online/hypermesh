using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Catalog endpoints (browse, search, package info, registry stats).
/// </summary>
public sealed class CatalogApi
{
    private readonly HttpApiClient _client;

    internal CatalogApi(HttpApiClient client) => _client = client;

    /// <summary>Browse packages.</summary>
    public Task<CatalogPackageList> BrowseAsync(string? query = null, int? page = null, CancellationToken ct = default)
    {
        var parts = new List<string>();
        if (query != null) parts.Add($"query={Uri.EscapeDataString(query)}");
        if (page.HasValue) parts.Add($"page={page.Value}");
        var qs = parts.Count > 0 ? $"?{string.Join("&", parts)}" : "";
        return _client.GetAsync<CatalogPackageList>($"/api/v1/catalog/browse{qs}", ct);
    }

    /// <summary>Search packages.</summary>
    public Task<CatalogSearchResults> SearchAsync(string query, CancellationToken ct = default)
        => _client.GetAsync<CatalogSearchResults>(
            $"/api/v1/catalog/search?query={Uri.EscapeDataString(query)}", ct);

    /// <summary>Get package info.</summary>
    public Task<CatalogPackageInfo> PackageInfoAsync(string name, CancellationToken ct = default)
        => _client.GetAsync<CatalogPackageInfo>(
            $"/api/v1/catalog/package/{Uri.EscapeDataString(name)}", ct);

    /// <summary>Get registry stats.</summary>
    public Task<CatalogRegistryStats> RegistryStatsAsync(CancellationToken ct = default)
        => _client.GetAsync<CatalogRegistryStats>("/api/v1/catalog/registry/stats", ct);
}
