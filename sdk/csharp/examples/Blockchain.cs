// Blockchain operations with the HyperMesh C# SDK.
//
// Run: dotnet run

using HyperMesh.Sdk;

using var client = new HyperMeshClient();

// Get chain height
var h = await client.Blockchain.HeightAsync();
Console.WriteLine($"Blockchain height: {h.Height}");

// Fetch the genesis block
var genesis = await client.Blockchain.BlockAsync(0);
Console.WriteLine($"\nGenesis block:");
Console.WriteLine($"  Index: {genesis.Index}");
Console.WriteLine($"  Hash: {genesis.Hash}");
Console.WriteLine($"  Previous hash: {genesis.PreviousHash}");
Console.WriteLine($"  Timestamp: {genesis.Timestamp}");

// Fetch the latest block
if (h.Height > 0)
{
    var latest = await client.Blockchain.BlockAsync(h.Height - 1);
    Console.WriteLine($"\nLatest block (index {latest.Index}):");
    Console.WriteLine($"  Hash: {latest.Hash}");
}

// Validate the chain
var result = await client.Blockchain.ValidateAsync();
Console.WriteLine($"\nBlockchain valid: {result.Valid}");
Console.WriteLine($"Blocks checked: {result.BlockCount}");
if (result.Errors.Count > 0)
{
    Console.WriteLine($"Errors: {string.Join(", ", result.Errors)}");
}
