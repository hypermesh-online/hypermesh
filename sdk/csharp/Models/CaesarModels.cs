using System.Text.Json.Serialization;

namespace HyperMesh.Sdk.Models;

public sealed record CaesarWalletInfo
{
    [JsonPropertyName("balance_grams")]
    public double BalanceGrams { get; init; }

    [JsonPropertyName("balance_usd")]
    public double BalanceUsd { get; init; }

    [JsonPropertyName("tier")]
    public string Tier { get; init; } = string.Empty;

    [JsonPropertyName("node_id")]
    public string NodeId { get; init; } = string.Empty;
}

public sealed record CaesarBalance
{
    [JsonPropertyName("gold_grams")]
    public double GoldGrams { get; init; }

    [JsonPropertyName("usd_equivalent")]
    public double UsdEquivalent { get; init; }

    [JsonPropertyName("tier")]
    public string Tier { get; init; } = string.Empty;
}

public sealed record CaesarTransaction
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("from")]
    public string From { get; init; } = string.Empty;

    [JsonPropertyName("to")]
    public string To { get; init; } = string.Empty;

    [JsonPropertyName("amount_grams")]
    public double AmountGrams { get; init; }

    [JsonPropertyName("fee")]
    public double Fee { get; init; }

    [JsonPropertyName("status")]
    public string Status { get; init; } = string.Empty;

    [JsonPropertyName("timestamp")]
    public long Timestamp { get; init; }
}

public sealed record CaesarTransactionList
{
    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("transactions")]
    public List<CaesarTransaction> Transactions { get; init; } = [];
}

public sealed record CaesarRewardInfo
{
    [JsonPropertyName("total_earned")]
    public double TotalEarned { get; init; }

    [JsonPropertyName("pending")]
    public double Pending { get; init; }

    [JsonPropertyName("tier_multiplier")]
    public double TierMultiplier { get; init; }
}

public sealed record CaesarRouteRequest
{
    [JsonPropertyName("destination")]
    public string Destination { get; init; } = string.Empty;

    [JsonPropertyName("amount_grams")]
    public double AmountGrams { get; init; }
}

public sealed record CaesarRouteResult
{
    [JsonPropertyName("packet_id")]
    public string PacketId { get; init; } = string.Empty;

    [JsonPropertyName("status")]
    public string Status { get; init; } = string.Empty;

    [JsonPropertyName("fee")]
    public double Fee { get; init; }
}

public sealed record CaesarGovernorParams
{
    [JsonPropertyName("velocity")]
    public double Velocity { get; init; }

    [JsonPropertyName("fee_rate")]
    public double FeeRate { get; init; }

    [JsonPropertyName("demurrage_rate")]
    public double DemurrageRate { get; init; }
}
