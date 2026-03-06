using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record CatalogPackage
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("version")]
    public string Version { get; init; } = string.Empty;

    [JsonPropertyName("description")]
    public string Description { get; init; } = string.Empty;

    [JsonPropertyName("author")]
    public string Author { get; init; } = string.Empty;

    [JsonPropertyName("downloads")]
    public long Downloads { get; init; }
}

public sealed record CatalogPackageList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("packages")]
    public List<CatalogPackage> Packages { get; init; } = [];
}

public sealed record CatalogSearchResult
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("version")]
    public string Version { get; init; } = string.Empty;

    [JsonPropertyName("description")]
    public string Description { get; init; } = string.Empty;

    [JsonPropertyName("relevance")]
    public double Relevance { get; init; }
}

public sealed record CatalogSearchResults
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("results")]
    public List<CatalogSearchResult> Results { get; init; } = [];
}

public sealed record CatalogPackageInfo
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("version")]
    public string Version { get; init; } = string.Empty;

    [JsonPropertyName("description")]
    public string Description { get; init; } = string.Empty;

    [JsonPropertyName("author")]
    public string Author { get; init; } = string.Empty;

    [JsonPropertyName("downloads")]
    public long Downloads { get; init; }
}

public sealed record CatalogRegistryStats
{
    [JsonPropertyName("package_count")]
    public int PackageCount { get; init; }

    [JsonPropertyName("publisher_count")]
    public int PublisherCount { get; init; }

    [JsonPropertyName("total_downloads")]
    public long TotalDownloads { get; init; }
}
