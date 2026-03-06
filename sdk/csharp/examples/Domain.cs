// Domain operations with the HyperMesh C# SDK.
//
// Run: dotnet run

using HyperMesh.Sdk;

using var client = new HyperMeshClient();

// List registered domains
var domains = await client.Domain.ListAsync();
Console.WriteLine($"Domains: {domains.Count}");
foreach (var d in domains.Domains)
{
    Console.WriteLine($"  {d.Name} (privacy: {d.Privacy}, owner: {d.Owner})");
}

// Register a new domain
Console.WriteLine("\nRegistering domain 'testapp' with Private privacy...");
var reg = await client.Domain.RegisterAsync("testapp", "Private");
Console.WriteLine($"Registered: {reg.Name} success={reg.Success}");

// Join a domain
Console.WriteLine("\nJoining domain 'testapp'...");
var join = await client.Domain.JoinAsync("testapp");
Console.WriteLine($"Joined: {join.Name} success={join.Success}");

// Dashboard and config
var info = await client.Dashboard.InfoAsync();
Console.WriteLine($"\nDashboard info: {info}");

var dashboards = await client.Dashboard.ListAsync();
Console.WriteLine($"Dashboards: {dashboards.Count}");

var config = await client.Config.ShowAsync();
Console.WriteLine($"Config: {config}");

// Assets
var assets = await client.Asset.ListAsync();
Console.WriteLine($"\nAssets: {assets.Count}");

// Topology
var topo = await client.Topology.InfoAsync();
Console.WriteLine($"\nTopology: node={topo.NodeId}");

var neighbors = await client.Topology.NeighborsAsync();
Console.WriteLine($"Neighbors: {neighbors.Count}");
