using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Engauge endpoints (capacity, traffic, marketplace, node metrics, leases).
/// </summary>
public sealed class EngaugeApi
{
    private readonly HttpApiClient _client;

    internal EngaugeApi(HttpApiClient client) => _client = client;

    /// <summary>Get capacity metrics.</summary>
    public Task<EngaugeCapacityMetrics> CapacityAsync(CancellationToken ct = default)
        => _client.GetAsync<EngaugeCapacityMetrics>("/api/v1/engauge/capacity", ct);

    /// <summary>Get traffic metrics.</summary>
    public Task<EngaugeTrafficMetrics> TrafficAsync(CancellationToken ct = default)
        => _client.GetAsync<EngaugeTrafficMetrics>("/api/v1/engauge/traffic", ct);

    /// <summary>List marketplace listings.</summary>
    public Task<EngaugeListingList> MarketplaceListingsAsync(CancellationToken ct = default)
        => _client.GetAsync<EngaugeListingList>("/api/v1/engauge/marketplace/listings", ct);

    /// <summary>Get node metrics.</summary>
    public Task<EngaugeNodeMetrics> NodeMetricsAsync(CancellationToken ct = default)
        => _client.GetAsync<EngaugeNodeMetrics>("/api/v1/engauge/node/metrics", ct);

    /// <summary>List leases.</summary>
    public Task<EngaugeLeaseList> LeasesAsync(CancellationToken ct = default)
        => _client.GetAsync<EngaugeLeaseList>("/api/v1/engauge/leases", ct);
}
