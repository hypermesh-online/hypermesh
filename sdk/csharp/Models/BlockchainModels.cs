using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record BlockchainHeight
{
    [JsonPropertyName("height")]
    public long Height { get; init; }
}

public sealed record Block
{
    [JsonPropertyName("index")]
    public long Index { get; init; }

    [JsonPropertyName("timestamp")]
    public long Timestamp { get; init; }

    [JsonPropertyName("hash")]
    public string Hash { get; init; } = string.Empty;

    [JsonPropertyName("previous_hash")]
    public string PreviousHash { get; init; } = string.Empty;

    [JsonPropertyName("data")]
    public string? Data { get; init; }
}

public sealed record ValidationResult
{
    [JsonPropertyName("valid")]
    public bool Valid { get; init; }

    [JsonPropertyName("errors")]
    public List<string> Errors { get; init; } = [];

    [JsonPropertyName("block_count")]
    public long BlockCount { get; init; }
}
