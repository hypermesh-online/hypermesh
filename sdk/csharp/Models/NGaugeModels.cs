using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record NGaugeCapacityMetrics
{
    [JsonPropertyName("bytes_served")]
    public long BytesServed { get; init; }

    [JsonPropertyName("compute_delivered")]
    public double ComputeDelivered { get; init; }

    [JsonPropertyName("storage")]
    public long Storage { get; init; }

    [JsonPropertyName("bandwidth")]
    public double Bandwidth { get; init; }

    [JsonPropertyName("uptime")]
    public double Uptime { get; init; }
}

public sealed record NGaugeTrafficMetrics
{
    [JsonPropertyName("organic_ratio")]
    public double OrganicRatio { get; init; }

    [JsonPropertyName("speculative_ratio")]
    public double SpeculativeRatio { get; init; }

    [JsonPropertyName("total_requests")]
    public long TotalRequests { get; init; }
}

public sealed record NGaugeListing
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("resource_type")]
    public string ResourceType { get; init; } = string.Empty;

    [JsonPropertyName("price")]
    public double Price { get; init; }
}

public sealed record NGaugeListingList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("listings")]
    public List<NGaugeListing> Listings { get; init; } = [];
}

public sealed record NGaugeNodeMetrics
{
    [JsonPropertyName("activity_score")]
    public double ActivityScore { get; init; }

    [JsonPropertyName("receipts")]
    public long Receipts { get; init; }

    [JsonPropertyName("bandwidth")]
    public double Bandwidth { get; init; }
}

public sealed record NGaugeLease
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("resource_type")]
    public string ResourceType { get; init; } = string.Empty;

    [JsonPropertyName("status")]
    public string Status { get; init; } = string.Empty;
}

public sealed record NGaugeLeaseList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("leases")]
    public List<NGaugeLease> Leases { get; init; } = [];
}
