using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Caesar EVP endpoints (wallet, transactions, rewards, routing, governor).
/// </summary>
public sealed class CaesarApi
{
    private readonly HttpApiClient _client;

    internal CaesarApi(HttpApiClient client) => _client = client;

    /// <summary>Get wallet info.</summary>
    public Task<CaesarWalletInfo> WalletAsync(CancellationToken ct = default)
        => _client.GetAsync<CaesarWalletInfo>("/api/v1/caesar/wallet", ct);

    /// <summary>Get balance.</summary>
    public Task<CaesarBalance> BalanceAsync(CancellationToken ct = default)
        => _client.GetAsync<CaesarBalance>("/api/v1/caesar/balance", ct);

    /// <summary>List transactions.</summary>
    public Task<CaesarTransactionList> TransactionsAsync(int? limit = null, CancellationToken ct = default)
    {
        var path = limit.HasValue
            ? $"/api/v1/caesar/transactions?limit={limit.Value}"
            : "/api/v1/caesar/transactions";
        return _client.GetAsync<CaesarTransactionList>(path, ct);
    }

    /// <summary>Get reward info.</summary>
    public Task<CaesarRewardInfo> RewardsAsync(CancellationToken ct = default)
        => _client.GetAsync<CaesarRewardInfo>("/api/v1/caesar/rewards", ct);

    /// <summary>Route an EVP packet.</summary>
    public Task<CaesarRouteResult> RoutePacketAsync(string destination, double amountGrams, CancellationToken ct = default)
        => _client.PostAsync<CaesarRouteResult>(
            "/api/v1/caesar/route",
            new CaesarRouteRequest { Destination = destination, AmountGrams = amountGrams },
            ct);

    /// <summary>Get governor parameters.</summary>
    public Task<CaesarGovernorParams> GovernorParamsAsync(CancellationToken ct = default)
        => _client.GetAsync<CaesarGovernorParams>("/api/v1/caesar/governor/params", ct);
}
