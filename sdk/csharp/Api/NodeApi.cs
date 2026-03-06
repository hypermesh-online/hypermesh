using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Node status and health endpoints.
/// </summary>
public sealed class NodeApi
{
    private readonly HttpApiClient _client;

    internal NodeApi(HttpApiClient client) => _client = client;

    /// <summary>Get full node status.</summary>
    public Task<NodeStatus> StatusAsync(CancellationToken ct = default)
        => _client.GetAsync<NodeStatus>("/api/v1/status", ct);

    /// <summary>Ping the node for liveness.</summary>
    public Task<PingResponse> PingAsync(CancellationToken ct = default)
        => _client.GetAsync<PingResponse>("/api/v1/ping", ct);
}
