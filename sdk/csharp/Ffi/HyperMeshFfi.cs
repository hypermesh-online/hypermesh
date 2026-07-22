using System;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace HyperMesh.Sdk.Ffi;

/// <summary>
/// Native FFI client using P/Invoke to libhypermesh_ffi.
/// Provides the same API surface as <see cref="HyperMeshClient"/> but
/// communicates over a Unix domain socket via the native C library
/// instead of HTTP.
/// </summary>
public sealed class HyperMeshFfi : IDisposable
{
    // -------------------------------------------------------------------
    // P/Invoke declarations — matches hypermesh.h exactly
    // -------------------------------------------------------------------

    private const string Lib = "hypermesh_ffi";

    // -- Lifecycle ------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_connect(string? socket_path);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern void hypermesh_disconnect(IntPtr client);

    // -- Raw RPC --------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_call(IntPtr client, string method, string params_json);

    // -- Node -----------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_status(IntPtr client);

    // -- DNS ------------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_dns_resolve(IntPtr client, string name);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_dns_list(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_dns_register(IntPtr client, string name, string addr);

    // -- Network --------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_peers(IntPtr client);

    // -- Blockchain -----------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_blockchain_height(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_blockchain_block(IntPtr client, ulong index);

    // -- Topology -------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_topology_info(IntPtr client);

    // -- Assets ---------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_asset_list(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_asset_store(IntPtr client, string file_path);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_asset_fetch(IntPtr client, string asset_id, string output_path);

    // -- Domains --------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_domain_list(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_domain_register(IntPtr client, string name, string privacy);

    // -- Dashboards -----------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_dashboard_list(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_dashboard_deploy(IntPtr client, string path);

    // -- Config ---------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_config_show(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_config_get(IntPtr client, string key);

    // -- Caesar EVP -----------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_caesar_wallet(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_caesar_balance(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_caesar_transactions(IntPtr client, uint limit);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_caesar_rewards(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_caesar_route_packet(IntPtr client, string destination, double amount_grams);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_caesar_governor_params(IntPtr client);

    // -- TrustChain -----------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_trustchain_certificates(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_trustchain_issue(IntPtr client, string subject, string scope);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_trustchain_validate(IntPtr client, string cert_pem);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_trustchain_revoke(IntPtr client, string cert_id);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_trustchain_dns_zones(IntPtr client);

    // -- NGauge --------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_ngauge_capacity(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_ngauge_traffic(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_ngauge_marketplace(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_ngauge_node_metrics(IntPtr client);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_ngauge_leases(IntPtr client);

    // -- Catalog --------------------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_catalog_browse(IntPtr client, string? query, uint page);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_catalog_search(IntPtr client, string query);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr hypermesh_catalog_package_info(IntPtr client, string name);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_catalog_registry_stats(IntPtr client);

    // -- Memory management ----------------------------------------------
    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern void hypermesh_free_string(IntPtr s);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr hypermesh_last_error(IntPtr client);

    // -------------------------------------------------------------------
    // Instance state
    // -------------------------------------------------------------------

    private IntPtr _client;
    private bool _disposed;
    private static bool _resolverRegistered;
    private static readonly object ResolverLock = new();

    /// <summary>
    /// Whether the FFI client is connected to a daemon.
    /// </summary>
    public bool IsConnected => _client != IntPtr.Zero;

    /// <summary>
    /// Create a new FFI client and connect to the HyperMesh daemon.
    /// </summary>
    /// <param name="socketPath">
    /// Unix socket path, or null for the default 3-tier fallback
    /// ($HYPERMESH_SOCK / $XDG_RUNTIME_DIR / ~/.hypermesh).
    /// </param>
    /// <param name="libPath">
    /// Explicit path to the native library file, or null to use
    /// the default resolver (env var / system paths / cargo target).
    /// </param>
    public HyperMeshFfi(string? socketPath = null, string? libPath = null)
    {
        EnsureResolverRegistered(libPath);

        _client = hypermesh_connect(socketPath);
        if (_client == IntPtr.Zero)
        {
            var err = GetLastErrorMessage();
            throw new HyperMeshFfiException(
                $"Failed to connect to HyperMesh daemon: {err ?? "unknown error"}");
        }
    }

    // -------------------------------------------------------------------
    // Core
    // -------------------------------------------------------------------

    /// <summary>
    /// Send a raw JSON-RPC call to the daemon.
    /// </summary>
    public JsonDocument Call(string method, string paramsJson)
    {
        ArgumentNullException.ThrowIfNull(method);
        ArgumentNullException.ThrowIfNull(paramsJson);
        return InvokeJson(c => hypermesh_call(c, method, paramsJson));
    }

    /// <summary>Fetch current node status.</summary>
    public JsonDocument Status()
        => InvokeJson(hypermesh_status);

    // -------------------------------------------------------------------
    // DNS
    // -------------------------------------------------------------------

    /// <summary>Resolve a DNS name to an address.</summary>
    public string DnsResolve(string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        return InvokeString(c => hypermesh_dns_resolve(c, name));
    }

    /// <summary>List all registered DNS entries.</summary>
    public JsonDocument DnsList()
        => InvokeJson(hypermesh_dns_list);

    /// <summary>Register a DNS name pointing to the given address.</summary>
    public JsonDocument DnsRegister(string name, string addr)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(addr);
        return InvokeJson(c => hypermesh_dns_register(c, name, addr));
    }

    // -------------------------------------------------------------------
    // Network
    // -------------------------------------------------------------------

    /// <summary>List connected peers.</summary>
    public JsonDocument Peers()
        => InvokeJson(hypermesh_peers);

    // -------------------------------------------------------------------
    // Blockchain
    // -------------------------------------------------------------------

    /// <summary>Get the current blockchain height.</summary>
    public JsonDocument BlockchainHeight()
        => InvokeJson(hypermesh_blockchain_height);

    /// <summary>Get a block by index.</summary>
    public JsonDocument BlockchainBlock(ulong index)
        => InvokeJson(c => hypermesh_blockchain_block(c, index));

    // -------------------------------------------------------------------
    // Topology
    // -------------------------------------------------------------------

    /// <summary>Get this node's topology info.</summary>
    public JsonDocument TopologyInfo()
        => InvokeJson(hypermesh_topology_info);

    // -------------------------------------------------------------------
    // Assets
    // -------------------------------------------------------------------

    /// <summary>List all stored assets.</summary>
    public JsonDocument AssetList()
        => InvokeJson(hypermesh_asset_list);

    /// <summary>Store a file as a HyperMesh asset.</summary>
    public JsonDocument AssetStore(string filePath)
    {
        ArgumentNullException.ThrowIfNull(filePath);
        return InvokeJson(c => hypermesh_asset_store(c, filePath));
    }

    /// <summary>Fetch an asset by ID and write it to the output path.</summary>
    public string AssetFetch(string assetId, string outputPath)
    {
        ArgumentNullException.ThrowIfNull(assetId);
        ArgumentNullException.ThrowIfNull(outputPath);
        return InvokeString(c => hypermesh_asset_fetch(c, assetId, outputPath));
    }

    // -------------------------------------------------------------------
    // Domains
    // -------------------------------------------------------------------

    /// <summary>List registered domains.</summary>
    public JsonDocument DomainList()
        => InvokeJson(hypermesh_domain_list);

    /// <summary>Register a domain with a name and privacy mode.</summary>
    public JsonDocument DomainRegister(string name, string privacy)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(privacy);
        return InvokeJson(c => hypermesh_domain_register(c, name, privacy));
    }

    // -------------------------------------------------------------------
    // Dashboards
    // -------------------------------------------------------------------

    /// <summary>List deployed dashboards.</summary>
    public JsonDocument DashboardList()
        => InvokeJson(hypermesh_dashboard_list);

    /// <summary>Deploy a dashboard from the given directory path.</summary>
    public JsonDocument DashboardDeploy(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        return InvokeJson(c => hypermesh_dashboard_deploy(c, path));
    }

    // -------------------------------------------------------------------
    // Config
    // -------------------------------------------------------------------

    /// <summary>Show the full daemon configuration.</summary>
    public JsonDocument ConfigShow()
        => InvokeJson(hypermesh_config_show);

    /// <summary>Get a single configuration value by key.</summary>
    public JsonDocument ConfigGet(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return InvokeJson(c => hypermesh_config_get(c, key));
    }

    // -------------------------------------------------------------------
    // Caesar EVP
    // -------------------------------------------------------------------

    /// <summary>Get the caller's Caesar wallet info.</summary>
    public JsonDocument CaesarWallet()
        => InvokeJson(hypermesh_caesar_wallet);

    /// <summary>Get the current Caesar balance.</summary>
    public JsonDocument CaesarBalance()
        => InvokeJson(hypermesh_caesar_balance);

    /// <summary>List recent Caesar transactions.</summary>
    /// <param name="limit">Maximum number of transactions (0 = default).</param>
    public JsonDocument CaesarTransactions(uint limit = 0)
        => InvokeJson(c => hypermesh_caesar_transactions(c, limit));

    /// <summary>Get accumulated Caesar rewards.</summary>
    public JsonDocument CaesarRewards()
        => InvokeJson(hypermesh_caesar_rewards);

    /// <summary>Route a Caesar EVP packet to a destination.</summary>
    public JsonDocument CaesarRoutePacket(string destination, double amountGrams)
    {
        ArgumentNullException.ThrowIfNull(destination);
        return InvokeJson(c => hypermesh_caesar_route_packet(c, destination, amountGrams));
    }

    /// <summary>Get current Caesar Governor parameters.</summary>
    public JsonDocument CaesarGovernorParams()
        => InvokeJson(hypermesh_caesar_governor_params);

    // -------------------------------------------------------------------
    // TrustChain
    // -------------------------------------------------------------------

    /// <summary>List all TrustChain certificates.</summary>
    public JsonDocument TrustChainCertificates()
        => InvokeJson(hypermesh_trustchain_certificates);

    /// <summary>Issue a new certificate for a subject and scope.</summary>
    public JsonDocument TrustChainIssue(string subject, string scope)
    {
        ArgumentNullException.ThrowIfNull(subject);
        ArgumentNullException.ThrowIfNull(scope);
        return InvokeJson(c => hypermesh_trustchain_issue(c, subject, scope));
    }

    /// <summary>Validate a PEM-encoded certificate.</summary>
    public JsonDocument TrustChainValidate(string certPem)
    {
        ArgumentNullException.ThrowIfNull(certPem);
        return InvokeJson(c => hypermesh_trustchain_validate(c, certPem));
    }

    /// <summary>Revoke a certificate by ID.</summary>
    public JsonDocument TrustChainRevoke(string certId)
    {
        ArgumentNullException.ThrowIfNull(certId);
        return InvokeJson(c => hypermesh_trustchain_revoke(c, certId));
    }

    /// <summary>List TrustChain DNS zones.</summary>
    public JsonDocument TrustChainDnsZones()
        => InvokeJson(hypermesh_trustchain_dns_zones);

    // -------------------------------------------------------------------
    // NGauge Analytics
    // -------------------------------------------------------------------

    /// <summary>Get current node capacity metrics.</summary>
    public JsonDocument NGaugeCapacity()
        => InvokeJson(hypermesh_ngauge_capacity);

    /// <summary>Get current traffic statistics.</summary>
    public JsonDocument NGaugeTraffic()
        => InvokeJson(hypermesh_ngauge_traffic);

    /// <summary>Get marketplace resource pool info.</summary>
    public JsonDocument NGaugeMarketplace()
        => InvokeJson(hypermesh_ngauge_marketplace);

    /// <summary>Get detailed node-level metrics.</summary>
    public JsonDocument NGaugeNodeMetrics()
        => InvokeJson(hypermesh_ngauge_node_metrics);

    /// <summary>Get active resource leases.</summary>
    public JsonDocument NGaugeLeases()
        => InvokeJson(hypermesh_ngauge_leases);

    // -------------------------------------------------------------------
    // Catalog Registry
    // -------------------------------------------------------------------

    /// <summary>Browse catalog packages.</summary>
    /// <param name="query">Search query, or null for all packages.</param>
    /// <param name="page">Page number (0-indexed).</param>
    public JsonDocument CatalogBrowse(string? query = null, uint page = 0)
        => InvokeJson(c => hypermesh_catalog_browse(c, query, page));

    /// <summary>Search catalog packages by query string.</summary>
    public JsonDocument CatalogSearch(string query)
    {
        ArgumentNullException.ThrowIfNull(query);
        return InvokeJson(c => hypermesh_catalog_search(c, query));
    }

    /// <summary>Get detailed info about a specific catalog package.</summary>
    public JsonDocument CatalogPackageInfo(string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        return InvokeJson(c => hypermesh_catalog_package_info(c, name));
    }

    /// <summary>Get catalog registry statistics.</summary>
    public JsonDocument CatalogRegistryStats()
        => InvokeJson(hypermesh_catalog_registry_stats);

    // -------------------------------------------------------------------
    // IDisposable
    // -------------------------------------------------------------------

    /// <summary>
    /// Disconnect from the daemon and release the native client handle.
    /// </summary>
    public void Dispose()
    {
        if (!_disposed)
        {
            if (_client != IntPtr.Zero)
            {
                hypermesh_disconnect(_client);
                _client = IntPtr.Zero;
            }
            _disposed = true;
        }
    }

    ~HyperMeshFfi()
    {
        Dispose();
    }

    // -------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    /// <summary>
    /// Call an FFI function that returns a heap-allocated JSON string.
    /// Frees the native string after copying. Throws on null return.
    /// </summary>
    private JsonDocument InvokeJson(Func<IntPtr, IntPtr> ffiCall)
    {
        ThrowIfDisposed();

        var raw = ffiCall(_client);
        if (raw == IntPtr.Zero)
        {
            var err = GetLastErrorMessage();
            throw new HyperMeshFfiException(err ?? "FFI call returned null");
        }

        try
        {
            var json = Marshal.PtrToStringAnsi(raw);
            if (string.IsNullOrEmpty(json))
                throw new HyperMeshFfiException("FFI call returned empty string");

            return JsonDocument.Parse(json);
        }
        finally
        {
            hypermesh_free_string(raw);
        }
    }

    /// <summary>
    /// Call an FFI function that returns a heap-allocated plain string.
    /// Frees the native string after copying. Throws on null return.
    /// </summary>
    private string InvokeString(Func<IntPtr, IntPtr> ffiCall)
    {
        ThrowIfDisposed();

        var raw = ffiCall(_client);
        if (raw == IntPtr.Zero)
        {
            var err = GetLastErrorMessage();
            throw new HyperMeshFfiException(err ?? "FFI call returned null");
        }

        try
        {
            var result = Marshal.PtrToStringAnsi(raw);
            return result ?? throw new HyperMeshFfiException("FFI call returned null string");
        }
        finally
        {
            hypermesh_free_string(raw);
        }
    }

    /// <summary>
    /// Read the thread-local error message from the native library.
    /// Returns null if no error is set. Does NOT free the pointer
    /// (the native side owns it).
    /// </summary>
    private string? GetLastErrorMessage()
    {
        var errPtr = hypermesh_last_error(_client);
        if (errPtr == IntPtr.Zero)
            return null;
        return Marshal.PtrToStringAnsi(errPtr);
    }

    private static void EnsureResolverRegistered(string? libPath)
    {
        if (_resolverRegistered && libPath == null)
            return;

        lock (ResolverLock)
        {
            if (!_resolverRegistered)
            {
                NativeLibraryResolver.Register(libPath);
                _resolverRegistered = true;
            }
        }
    }
}

/// <summary>
/// Exception thrown when an FFI call to the native HyperMesh library fails.
/// </summary>
public sealed class HyperMeshFfiException : Exception
{
    public HyperMeshFfiException(string message) : base(message) { }
    public HyperMeshFfiException(string message, Exception inner) : base(message, inner) { }
}
