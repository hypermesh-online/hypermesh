using HyperMesh.Sdk.Api;

namespace HyperMesh.Sdk;

/// <summary>
/// Client for the HyperMesh node HTTP REST API.
/// </summary>
public sealed class HyperMeshClient : IDisposable
{
    private const string DefaultBaseUrl = "http://localhost:9293";
    private const string DefaultCaesarUrl = "http://localhost:9294";
    private const string DefaultTrustChainUrl = "http://localhost:8444";
    private const string DefaultCatalogUrl = "http://localhost:9295";
    private const string DefaultEngaugeUrl = "http://localhost:9296";

    private readonly HttpApiClient _apiClient;
    private readonly HttpApiClient _caesarClient;
    private readonly HttpApiClient _trustChainClient;
    private readonly HttpApiClient _catalogClient;
    private readonly HttpApiClient _engaugeClient;
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

    /// <summary>Caesar EVP (wallet, transactions, rewards, routing).</summary>
    public CaesarApi Caesar { get; }

    /// <summary>TrustChain (certificates, validation, DNS zones).</summary>
    public TrustChainApi TrustChain { get; }

    /// <summary>Engauge (capacity, traffic, marketplace, metrics).</summary>
    public EngaugeApi Engauge { get; }

    /// <summary>Catalog (packages, search, registry).</summary>
    public CatalogApi Catalog { get; }

    /// <summary>
    /// Create a new HyperMesh client.
    /// </summary>
    /// <param name="baseUrl">BlockMatrix API base URL (default: http://localhost:9293).</param>
    /// <param name="caesarUrl">Caesar EVP API base URL (default: http://localhost:9294).</param>
    /// <param name="trustChainUrl">TrustChain API base URL (default: http://localhost:8444).</param>
    /// <param name="catalogUrl">Catalog API base URL (default: http://localhost:9295).</param>
    /// <param name="engaugeUrl">Engauge API base URL (default: http://localhost:9296).</param>
    /// <param name="httpClient">Optional HttpClient instance for custom configuration.</param>
    public HyperMeshClient(
        string baseUrl = DefaultBaseUrl,
        string caesarUrl = DefaultCaesarUrl,
        string trustChainUrl = DefaultTrustChainUrl,
        string catalogUrl = DefaultCatalogUrl,
        string engaugeUrl = DefaultEngaugeUrl,
        HttpClient? httpClient = null)
    {
        _apiClient = new HttpApiClient(baseUrl, httpClient);
        _caesarClient = new HttpApiClient(caesarUrl);
        _trustChainClient = new HttpApiClient(trustChainUrl);
        _catalogClient = new HttpApiClient(catalogUrl);
        _engaugeClient = new HttpApiClient(engaugeUrl);

        Node = new NodeApi(_apiClient);
        Blockchain = new BlockchainApi(_apiClient);
        Dns = new DnsApi(_apiClient);
        Network = new NetworkApi(_apiClient);
        Topology = new TopologyApi(_apiClient);
        Asset = new AssetApi(_apiClient);
        Dashboard = new DashboardApi(_apiClient);
        Config = new ConfigApi(_apiClient);
        Domain = new DomainApi(_apiClient);
        Caesar = new CaesarApi(_caesarClient);
        TrustChain = new TrustChainApi(_trustChainClient);
        Engauge = new EngaugeApi(_engaugeClient);
        Catalog = new CatalogApi(_catalogClient);
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _apiClient.Dispose();
            _caesarClient.Dispose();
            _trustChainClient.Dispose();
            _catalogClient.Dispose();
            _engaugeClient.Dispose();
            _disposed = true;
        }
    }
}
