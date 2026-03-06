using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Dashboard management endpoints.
/// </summary>
public sealed class DashboardApi
{
    private readonly HttpApiClient _client;

    internal DashboardApi(HttpApiClient client) => _client = client;

    /// <summary>List available dashboards.</summary>
    public Task<DashboardList> ListAsync(CancellationToken ct = default)
        => _client.GetAsync<DashboardList>("/api/v1/dashboard/list", ct);

    /// <summary>Get dashboard info.</summary>
    public Task<DashboardInfo> InfoAsync(CancellationToken ct = default)
        => _client.GetAsync<DashboardInfo>("/api/v1/dashboard/info", ct);
}
