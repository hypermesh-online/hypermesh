// Basic usage of the HyperMesh C# SDK.
//
// Run: dotnet run
// Requires a project reference to HyperMesh.Sdk.

using HyperMesh.Sdk;

using var client = new HyperMeshClient(); // http://localhost:9293

// Ping the node
var pong = await client.Node.PingAsync();
Console.WriteLine($"Ping: {pong.Pong}");

// Get node status
var status = await client.Node.StatusAsync();
Console.WriteLine($"Node ID: {status.NodeId}");
Console.WriteLine($"Chain height: {status.ChainHeight}");
Console.WriteLine($"Peers: {status.Peers}");
Console.WriteLine($"Privacy mode: {status.PrivacyMode}");
Console.WriteLine($"Uptime: {status.UptimeSecs} seconds");

// List connected peers
var peers = await client.Network.PeersAsync();
Console.WriteLine($"\nConnected peers: {peers.Count}");

// Error handling
try
{
    await client.Blockchain.BlockAsync(999999);
}
catch (HyperMeshException ex)
{
    Console.WriteLine($"\nExpected error for block 999999:");
    Console.WriteLine($"  Status: {ex.StatusCode}");
    Console.WriteLine($"  Message: {ex.Message}");
}
