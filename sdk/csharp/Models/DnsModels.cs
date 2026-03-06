using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record DnsList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("records")]
    public List<DnsRecord> Records { get; init; } = [];
}

public sealed record DnsRecord
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("address")]
    public string Address { get; init; } = string.Empty;
}

public sealed record DnsRegisterRequest
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("address")]
    public string Address { get; init; } = string.Empty;
}

public sealed record DnsRegisterResponse
{
    [JsonPropertyName("success")]
    public bool Success { get; init; }

    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;
}
