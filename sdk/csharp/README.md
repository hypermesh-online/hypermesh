# HyperMesh C# SDK

C# client library for the HyperMesh node HTTP REST API. Targets .NET 8.0 with zero external dependencies.

## Installation

Add a project reference or include in your solution:

```bash
dotnet add reference path/to/HyperMesh.Sdk.csproj
```

## Usage

```csharp
using HyperMesh.Sdk;

using var client = new HyperMeshClient("http://localhost:9293");

// Node status
var status = await client.Node.StatusAsync();
Console.WriteLine($"Node {status.NodeId} at height {status.ChainHeight}");

// Blockchain
var height = await client.Blockchain.HeightAsync();
var block = await client.Blockchain.BlockAsync(0);
var validation = await client.Blockchain.ValidateAsync();

// DNS
var records = await client.Dns.ListAsync();
await client.Dns.RegisterAsync("my-service", "fd00::1");
var resolved = await client.Dns.ResolveAsync("my-service");

// Network
var peers = await client.Network.PeersAsync();

// Topology
var info = await client.Topology.InfoAsync();
var neighbors = await client.Topology.NeighborsAsync();

// Assets
var assets = await client.Asset.ListAsync();

// Dashboards
var dashboards = await client.Dashboard.ListAsync();
var dashInfo = await client.Dashboard.InfoAsync();

// Config
var config = await client.Config.ShowAsync();
var value = await client.Config.GetAsync("privacy_mode");

// Domains
var domains = await client.Domain.ListAsync();
await client.Domain.RegisterAsync("my-domain", "Public");
await client.Domain.JoinAsync("other-domain", token: "invite-token");
```

## Error Handling

All API errors throw `HyperMeshException` which includes the HTTP status code and response body when available:

```csharp
try
{
    var block = await client.Blockchain.BlockAsync(999999);
}
catch (HyperMeshException ex)
{
    Console.WriteLine($"Error: {ex.Message}");
    Console.WriteLine($"Status: {ex.StatusCode}");
    Console.WriteLine($"Body: {ex.ResponseBody}");
}
```

## Custom HttpClient

Pass your own `HttpClient` for timeouts, proxies, or other configuration:

```csharp
var http = new HttpClient { Timeout = TimeSpan.FromSeconds(5) };
using var client = new HyperMeshClient("http://localhost:9293", http);
```

## API Reference

| Property | Methods | Description |
|----------|---------|-------------|
| `Node` | `StatusAsync`, `PingAsync` | Node health and status |
| `Blockchain` | `HeightAsync`, `BlockAsync`, `ValidateAsync` | Blockchain queries |
| `Dns` | `ListAsync`, `ResolveAsync`, `RegisterAsync` | DNS management |
| `Network` | `PeersAsync` | Peer connections |
| `Topology` | `InfoAsync`, `NeighborsAsync` | Matrix topology |
| `Asset` | `ListAsync` | Asset registry |
| `Dashboard` | `ListAsync`, `InfoAsync` | Dashboard management |
| `Config` | `ShowAsync`, `GetAsync` | Node configuration |
| `Domain` | `ListAsync`, `RegisterAsync`, `JoinAsync` | Domain management |
