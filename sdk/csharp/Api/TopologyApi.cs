using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Matrix topology endpoints.
/// </summary>
public sealed class TopologyApi
{
    private readonly HttpApiClient _client;

    internal TopologyApi(HttpApiClient client) => _client = client;

    /// <summary>Get this node's topology info (coordinate and ID).</summary>
    public Task<TopologyInfo> InfoAsync(CancellationToken ct = default)
        => _client.GetAsync<TopologyInfo>("/api/v1/topology/info", ct);

    /// <summary>List matrix neighbors.</summary>
    public Task<NeighborList> NeighborsAsync(CancellationToken ct = default)
        => _client.GetAsync<NeighborList>("/api/v1/topology/neighbors", ct);
}
