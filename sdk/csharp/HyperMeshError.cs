using System.Net;

namespace HyperMesh.Sdk;

/// <summary>
/// Exception thrown when a HyperMesh API call fails.
/// </summary>
public class HyperMeshException : Exception
{
    /// <summary>HTTP status code returned by the node, if available.</summary>
    public HttpStatusCode? StatusCode { get; }

    /// <summary>Raw response body, if available.</summary>
    public string? ResponseBody { get; }

    public HyperMeshException(string message)
        : base(message) { }

    public HyperMeshException(string message, Exception inner)
        : base(message, inner) { }

    public HyperMeshException(string message, HttpStatusCode statusCode, string? responseBody = null)
        : base(message)
    {
        StatusCode = statusCode;
        ResponseBody = responseBody;
    }
}
