using HyperMesh.Sdk.Api;

namespace HyperMesh.Sdk;

/// <summary>
/// Client for the HyperMesh node HTTP REST API.
/// </summary>
public sealed class HyperMeshClient : IDisposable
{
    private readonly HttpApiClient _apiClient;
    private bool _disposed;

    /// <summary>Node status and health.</summary>
    public NodeApi Node { get; }

    /// <summary>Blockchain queries.</summary>
    public BlockchainApi Blockchain { get; }

    /// <summary>DNS record management.</summary>
    public DnsApi Dns { get; }

    /// <summary>Network peer management.</summary>
    public NetworkApi Network { get; }

    /// <summary>Matrix topology.</summary>
    public TopologyApi Topology { get; }

    /// <summary>Asset management.</summary>
    public AssetApi Asset { get; }

    /// <summary>Dashboard management.</summary>
    public DashboardApi Dashboard { get; }

    /// <summary>Node configuration.</summary>
    public ConfigApi Config { get; }

    /// <summary>Domain registration and management.</summary>
    public DomainApi Domain { get; }

    /// <summary>
    /// Create a new HyperMesh client.
    /// </summary>
    /// <param name="baseUrl">Node API base URL (default: http://localhost:9293).</param>
    /// <param name="httpClient">Optional HttpClient instance for custom configuration.</param>
    public HyperMeshClient(string baseUrl = "http://localhost:9293", HttpClient? httpClient = null)
    {
        _apiClient = new HttpApiClient(baseUrl, httpClient);
        Node = new NodeApi(_apiClient);
        Blockchain = new BlockchainApi(_apiClient);
        Dns = new DnsApi(_apiClient);
        Network = new NetworkApi(_apiClient);
        Topology = new TopologyApi(_apiClient);
        Asset = new AssetApi(_apiClient);
        Dashboard = new DashboardApi(_apiClient);
        Config = new ConfigApi(_apiClient);
        Domain = new DomainApi(_apiClient);
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _apiClient.Dispose();
            _disposed = true;
        }
    }
}
