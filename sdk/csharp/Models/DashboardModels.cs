using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record DashboardList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("dashboards")]
    public List<DashboardSummary> Dashboards { get; init; } = [];
}

public sealed record DashboardSummary
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("scope")]
    public string Scope { get; init; } = string.Empty;
}

public sealed record DashboardInfo
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("scope")]
    public string Scope { get; init; } = string.Empty;

    [JsonPropertyName("version")]
    public string? Version { get; init; }

    [JsonPropertyName("description")]
    public string? Description { get; init; }
}
