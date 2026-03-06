using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record PeerList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("peers")]
    public List<PeerInfo> Peers { get; init; } = [];
}

public sealed record PeerInfo
{
    [JsonPropertyName("node_id")]
    public string NodeId { get; init; } = string.Empty;

    [JsonPropertyName("address")]
    public string Address { get; init; } = string.Empty;

    [JsonPropertyName("connected")]
    public bool Connected { get; init; }
}
