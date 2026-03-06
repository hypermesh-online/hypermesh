using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record TopologyInfo
{
    [JsonPropertyName("coordinate")]
    public Coordinate? Coordinate { get; init; }

    [JsonPropertyName("node_id")]
    public string NodeId { get; init; } = string.Empty;
}

public sealed record NeighborList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("neighbors")]
    public List<NeighborInfo> Neighbors { get; init; } = [];
}

public sealed record NeighborInfo
{
    [JsonPropertyName("node_id")]
    public string NodeId { get; init; } = string.Empty;

    [JsonPropertyName("coordinate")]
    public Coordinate? Coordinate { get; init; }

    [JsonPropertyName("distance")]
    public double Distance { get; init; }
}
