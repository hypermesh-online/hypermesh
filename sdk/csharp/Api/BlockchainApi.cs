using HyperMesh.Sdk.Models;

namespace HyperMesh.Sdk.Api;

/// <summary>
/// Blockchain query endpoints.
/// </summary>
public sealed class BlockchainApi
{
    private readonly HttpApiClient _client;

    internal BlockchainApi(HttpApiClient client) => _client = client;

    /// <summary>Get current blockchain height.</summary>
    public Task<BlockchainHeight> HeightAsync(CancellationToken ct = default)
        => _client.GetAsync<BlockchainHeight>("/api/v1/blockchain/height", ct);

    /// <summary>Get a block by index.</summary>
    public Task<Block> BlockAsync(long index, CancellationToken ct = default)
        => _client.GetAsync<Block>($"/api/v1/blockchain/block/{index}", ct);

    /// <summary>Validate the blockchain integrity.</summary>
    public Task<ValidationResult> ValidateAsync(CancellationToken ct = default)
        => _client.GetAsync<ValidationResult>("/api/v1/blockchain/validate", ct);
}
