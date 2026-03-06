// DNS operations with the HyperMesh C# SDK.
//
// Run: dotnet run

using HyperMesh.Sdk;

using var client = new HyperMeshClient();

// List existing DNS records
var dns = await client.Dns.ListAsync();
Console.WriteLine($"DNS records: {dns.Count}");
foreach (var record in dns.Records)
{
    Console.WriteLine($"  {record.Name} -> {record.Address}");
}

// Register a new record
Console.WriteLine("\nRegistering example.hypermesh -> ::1");
var reg = await client.Dns.RegisterAsync("example.hypermesh", "::1");
Console.WriteLine($"Registered: {reg}");

// Resolve the record
try
{
    var resolved = await client.Dns.ResolveAsync("example.hypermesh");
    Console.WriteLine($"Resolved: {resolved.Name} -> {resolved.Address}");
}
catch (HyperMeshException ex)
{
    Console.WriteLine($"Resolve failed: {ex.Message}");
}

// List records after registration
var updated = await client.Dns.ListAsync();
Console.WriteLine($"\nDNS records after registration: {updated.Count}");
foreach (var record in updated.Records)
{
    Console.WriteLine($"  {record.Name} -> {record.Address}");
}
