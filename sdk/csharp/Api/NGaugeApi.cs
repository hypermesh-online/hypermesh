using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// NGauge endpoints (capacity, traffic, marketplace, node metrics, leases).
/// </summary>
public sealed class NGaugeApi
{
    private readonly HttpApiClient _client;

    internal NGaugeApi(HttpApiClient client) => _client = client;

    /// <summary>Get capacity metrics.</summary>
    public Task<NGaugeCapacityMetrics> CapacityAsync(CancellationToken ct = default)
        => _client.GetAsync<NGaugeCapacityMetrics>("/api/v1/ngauge/capacity", ct);

    /// <summary>Get traffic metrics.</summary>
    public Task<NGaugeTrafficMetrics> TrafficAsync(CancellationToken ct = default)
        => _client.GetAsync<NGaugeTrafficMetrics>("/api/v1/ngauge/traffic", ct);

    /// <summary>List marketplace listings.</summary>
    public Task<NGaugeListingList> MarketplaceListingsAsync(CancellationToken ct = default)
        => _client.GetAsync<NGaugeListingList>("/api/v1/ngauge/marketplace/listings", ct);

    /// <summary>Get node metrics.</summary>
    public Task<NGaugeNodeMetrics> NodeMetricsAsync(CancellationToken ct = default)
        => _client.GetAsync<NGaugeNodeMetrics>("/api/v1/ngauge/node/metrics", ct);

    /// <summary>List leases.</summary>
    public Task<NGaugeLeaseList> LeasesAsync(CancellationToken ct = default)
        => _client.GetAsync<NGaugeLeaseList>("/api/v1/ngauge/leases", ct);
}
