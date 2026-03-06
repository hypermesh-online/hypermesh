using System.Text.Json.Serialization;
using System.Text.Json;

namespace HyperMesh.Sdk.Models;

public sealed record ConfigData
{
    [JsonPropertyName("config")]
    public Dictionary<string, JsonElement> Config { get; init; } = new();
}

public sealed record ConfigValue
{
    [JsonPropertyName("key")]
    public string Key { get; init; } = string.Empty;

    [JsonPropertyName("value")]
    public JsonElement Value { get; init; }
}
