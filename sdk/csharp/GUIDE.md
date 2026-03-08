# HyperMesh C# SDK Guide

## Installation

Add a project reference or package reference:

```bash
dotnet add package HyperMesh.Sdk
```

Requires .NET 8.0+. Uses `System.Text.Json` and `System.Net.Http` (no external dependencies).

## Quick Start

```csharp
using HyperMesh.Sdk;

using var client = new HyperMeshClient(); // defaults to https://localhost:8443
var status = await client.Node.StatusAsync();
Console.WriteLine($"{status.NodeId} height={status.ChainHeight}");
```

Custom endpoint:

```csharp
using var client = new HyperMeshClient("https://192.168.1.50:8443");
```

With a custom `HttpClient`:

```csharp
var httpClient = new HttpClient { Timeout = TimeSpan.FromSeconds(60) };
using var client = new HyperMeshClient("https://localhost:8443", httpClient);
```

## API Reference

All methods are async and accept an optional `CancellationToken`.

### Node

```csharp
// Get full node status
NodeStatus status = await client.Node.StatusAsync();
// Properties: NodeId, ChainHeight, Coordinate, Peers, PrivacyMode, UptimeSecs

// Ping the node
PingResponse pong = await client.Node.PingAsync();
// Properties: Pong (bool)
```

### Blockchain

```csharp
// Get current chain height
BlockchainHeight h = await client.Blockchain.HeightAsync();
// Properties: Height

// Get a specific block by index
Block block = await client.Blockchain.BlockAsync(0);
// Properties: Index, Timestamp, Hash, PreviousHash, Data

// Validate blockchain integrity
ValidationResult result = await client.Blockchain.ValidateAsync();
// Properties: Valid, Errors, BlockCount
```

### DNS

```csharp
// List all DNS records
DnsList dns = await client.Dns.ListAsync();
// Properties: Count, Records (List<DnsRecord>)
// DnsRecord: Name, Address

// Resolve a name
DnsRecord record = await client.Dns.ResolveAsync("trust.hypermesh");

// Register a new DNS record
DnsRegisterResponse resp = await client.Dns.RegisterAsync("mynode.hypermesh", "::1");
```

### Assets

```csharp
// List all registered assets
AssetList assets = await client.Asset.ListAsync();
// Properties: Count, Assets (List<AssetEntry>)
```

### Domain

```csharp
// List registered domains
DomainList domains = await client.Domain.ListAsync();
// Properties: Count, Domains (List<DomainInfo>)
// DomainInfo: Name, Privacy, Owner

// Register a new domain
DomainRegisterResponse reg = await client.Domain.RegisterAsync("myapp", "Private");

// Join an existing domain
DomainJoinResponse join = await client.Domain.JoinAsync("myapp");
// With invitation token:
DomainJoinResponse joinPrivate = await client.Domain.JoinAsync("myapp", "token-here");
```

### Dashboard

```csharp
// List available dashboards
DashboardList dashboards = await client.Dashboard.ListAsync();
// Properties: Count, Dashboards (List<DashboardEntry>)

// Get dashboard info
DashboardInfo info = await client.Dashboard.InfoAsync();
```

### Config

```csharp
// Show full node configuration
ConfigData config = await client.Config.ShowAsync();

// Get a specific config value
ConfigValue val = await client.Config.GetAsync("privacy_mode");
// Properties: Key, Value
```

### Network

```csharp
// List connected peers
PeerList peers = await client.Network.PeersAsync();
// Properties: Count, Peers (List<PeerInfo>)
```

### Topology

```csharp
// Get this node's position in the Block-MATRIX
TopologyInfo topo = await client.Topology.InfoAsync();
// Properties: NodeId, Coordinate

// List matrix neighbors
NeighborList neighbors = await client.Topology.NeighborsAsync();
// Properties: Count, Neighbors (List<NeighborInfo>)
```

## Error Handling

All methods throw `HyperMeshException` on failure:

```csharp
using HyperMesh.Sdk;

using var client = new HyperMeshClient();

try
{
    var block = await client.Blockchain.BlockAsync(999999);
}
catch (HyperMeshException ex)
{
    Console.WriteLine($"Status: {ex.StatusCode}");
    Console.WriteLine($"Body: {ex.ResponseBody}");
    Console.WriteLine($"Message: {ex.Message}");
}
```

`HyperMeshException` properties:
- `StatusCode` (`HttpStatusCode?`) -- HTTP status code, null for connection errors
- `ResponseBody` (`string?`) -- raw response body
- `Message` (`string`) -- human-readable error description

## Disposal

`HyperMeshClient` implements `IDisposable`. Always wrap in a `using` statement
or call `Dispose()` to release the underlying `HttpClient`:

```csharp
using var client = new HyperMeshClient();
// ... use client ...
// Disposed automatically at end of scope
```
