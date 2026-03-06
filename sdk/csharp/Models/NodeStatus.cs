using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record NodeStatus
{
    [JsonPropertyName("chain_height")]
    public long ChainHeight { get; init; }

    [JsonPropertyName("coordinate")]
    public Coordinate? Coordinate { get; init; }

    [JsonPropertyName("node_id")]
    public string NodeId { get; init; } = string.Empty;

    [JsonPropertyName("peers")]
    public int Peers { get; init; }

    [JsonPropertyName("privacy_mode")]
    public string PrivacyMode { get; init; } = string.Empty;

    [JsonPropertyName("uptime_secs")]
    public long UptimeSecs { get; init; }
}

public sealed record Coordinate
{
    [JsonPropertyName("x")]
    public double X { get; init; }

    [JsonPropertyName("y")]
    public double Y { get; init; }

    [JsonPropertyName("z")]
    public double Z { get; init; }
}

public sealed record PingResponse
{
    [JsonPropertyName("status")]
    public string Status { get; init; } = string.Empty;

    [JsonPropertyName("timestamp")]
    public long Timestamp { get; init; }
}
