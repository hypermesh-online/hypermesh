using System.Net;
using System.Net.Http.Json;
using System.Text.Json;

namespace HyperMesh.Sdk;

/// <summary>
/// Low-level HTTP wrapper for the HyperMesh node REST API.
/// </summary>
internal sealed class HttpApiClient : IDisposable
{
    /// <summary>
    /// Phase K.2 — header used to ship the capability token on HTTP
    /// requests to the gateway.
    /// </summary>
    public const string CapabilityTokenHeader = "X-HyperMesh-Capability";

    private readonly HttpClient _http;
    private readonly string _baseUrl;
    private string? _capabilityToken; // Phase K.2
    private bool _disposed;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };

    public HttpApiClient(string baseUrl, HttpClient? httpClient = null, string? capabilityToken = null)
    {
        _baseUrl = baseUrl.TrimEnd('/');
        _http = httpClient ?? new HttpClient();
        _capabilityToken = capabilityToken;
    }

    /// <summary>
    /// Phase K.2 — install or rotate the capability token. Pass null to clear.
    /// </summary>
    public void SetCapabilityToken(string? token) => _capabilityToken = token;

    /// <summary>Currently-installed capability token (or null).</summary>
    public string? GetCapabilityToken() => _capabilityToken;

    private void AttachCapabilityHeader(HttpRequestMessage request)
    {
        if (!string.IsNullOrEmpty(_capabilityToken))
        {
            request.Headers.Add(CapabilityTokenHeader, _capabilityToken);
        }
    }

    public async Task<T> GetAsync<T>(string path, CancellationToken ct = default)
    {
        var url = $"{_baseUrl}{path}";
        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        AttachCapabilityHeader(request);
        HttpResponseMessage response;
        try
        {
            response = await _http.SendAsync(request, ct).ConfigureAwait(false);
        }
        catch (HttpRequestException ex)
        {
            throw new HyperMeshException($"Failed to connect to {url}: {ex.Message}", ex);
        }

        return await HandleResponseAsync<T>(response, ct).ConfigureAwait(false);
    }

    public async Task<T> PostAsync<T>(string path, object body, CancellationToken ct = default)
    {
        var url = $"{_baseUrl}{path}";
        using var request = new HttpRequestMessage(HttpMethod.Post, url)
        {
            Content = JsonContent.Create(body, options: JsonOptions),
        };
        AttachCapabilityHeader(request);
        HttpResponseMessage response;
        try
        {
            response = await _http.SendAsync(request, ct).ConfigureAwait(false);
        }
        catch (HttpRequestException ex)
        {
            throw new HyperMeshException($"Failed to connect to {url}: {ex.Message}", ex);
        }

        return await HandleResponseAsync<T>(response, ct).ConfigureAwait(false);
    }

    public async Task PostAsync(string path, object body, CancellationToken ct = default)
    {
        var url = $"{_baseUrl}{path}";
        using var request = new HttpRequestMessage(HttpMethod.Post, url)
        {
            Content = JsonContent.Create(body, options: JsonOptions),
        };
        AttachCapabilityHeader(request);
        HttpResponseMessage response;
        try
        {
            response = await _http.SendAsync(request, ct).ConfigureAwait(false);
        }
        catch (HttpRequestException ex)
        {
            throw new HyperMeshException($"Failed to connect to {url}: {ex.Message}", ex);
        }

        await EnsureSuccessAsync(response, ct).ConfigureAwait(false);
    }

    private static async Task<T> HandleResponseAsync<T>(HttpResponseMessage response, CancellationToken ct)
    {
        var body = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);

        if (!response.IsSuccessStatusCode)
        {
            throw new HyperMeshException(
                $"API returned {(int)response.StatusCode} {response.ReasonPhrase}",
                response.StatusCode,
                body);
        }

        try
        {
            var result = JsonSerializer.Deserialize<T>(body, JsonOptions);
            return result ?? throw new HyperMeshException("API returned null response body");
        }
        catch (JsonException ex)
        {
            throw new HyperMeshException($"Failed to deserialize response: {ex.Message}", ex);
        }
    }

    private static async Task EnsureSuccessAsync(HttpResponseMessage response, CancellationToken ct)
    {
        if (!response.IsSuccessStatusCode)
        {
            var body = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            throw new HyperMeshException(
                $"API returned {(int)response.StatusCode} {response.ReasonPhrase}",
                response.StatusCode,
                body);
        }
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _http.Dispose();
            _disposed = true;
        }
    }
}
