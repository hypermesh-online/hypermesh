using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record DomainList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("domains")]
    public List<DomainInfo> Domains { get; init; } = [];
}

public sealed record DomainInfo
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("privacy")]
    public string Privacy { get; init; } = string.Empty;

    [JsonPropertyName("owner")]
    public string? Owner { get; init; }
}

public sealed record DomainRegisterRequest
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("privacy")]
    public string Privacy { get; init; } = string.Empty;
}

public sealed record DomainJoinRequest
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("token")]
    public string? Token { get; init; }
}

public sealed record DomainRegisterResponse
{
    [JsonPropertyName("success")]
    public bool Success { get; init; }

    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;
}

public sealed record DomainJoinResponse
{
    [JsonPropertyName("success")]
    public bool Success { get; init; }

    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;
}
