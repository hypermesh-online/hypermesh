using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record AssetList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("assets")]
    public List<AssetInfo> Assets { get; init; } = [];
}

public sealed record AssetInfo
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("asset_type")]
    public string AssetType { get; init; } = string.Empty;

    [JsonPropertyName("state")]
    public string State { get; init; } = string.Empty;

    [JsonPropertyName("name")]
    public string? Name { get; init; }
}
