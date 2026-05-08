export class HyperMeshError extends Error {
  public readonly status: number;
  public readonly body: string;

  constructor(message: string, status: number, body: string) {
    super(message);
    this.name = "HyperMeshError";
    this.status = status;
    this.body = body;
  }
}

/**
 * Phase K.2 — capability token attached to a client.
 *
 * When the HyperMesh daemon is configured for token enforcement, every
 * request must carry a base64-encoded `CapabilityToken` (issued via
 * `auth.create_session`). The token is sent as the
 * `X-HyperMesh-Capability` header and is also embedded in JSON-RPC
 * payloads forwarded to the daemon.
 *
 * Pre-K.2 daemons (alpha-default inert) ignore the token entirely, so
 * SDK clients without a token continue to work against unconfigured
 * daemons. Token-enforcing daemons reject untokened requests with
 * `CAPABILITY_DENIED` (-32004).
 */
export type CapabilityToken = string;

/** Header used to ship the capability token on HTTP requests. */
export const CAPABILITY_TOKEN_HEADER = "X-HyperMesh-Capability";

export class HttpClient {
  private readonly baseUrl: string;
  private capabilityToken: CapabilityToken | null;

  constructor(baseUrl: string, capabilityToken: CapabilityToken | null = null) {
    // Strip trailing slash
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.capabilityToken = capabilityToken;
  }

  /** Phase K.2 — install or rotate the capability token. */
  setCapabilityToken(token: CapabilityToken | null): void {
    this.capabilityToken = token;
  }

  /** Currently-installed capability token (or null). */
  getCapabilityToken(): CapabilityToken | null {
    return this.capabilityToken;
  }

  /** Build a base header set, augmented with the capability token if present. */
  private buildHeaders(extra: Record<string, string>): Record<string, string> {
    const out: Record<string, string> = { ...extra };
    if (this.capabilityToken) {
      out[CAPABILITY_TOKEN_HEADER] = this.capabilityToken;
    }
    return out;
  }

  async get<T>(path: string): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    let response: Response;

    try {
      response = await fetch(url, {
        method: "GET",
        headers: this.buildHeaders({ Accept: "application/json" }),
      });
    } catch (err) {
      throw new HyperMeshError(
        `Request failed: ${err instanceof Error ? err.message : String(err)}`,
        0,
        "",
      );
    }

    if (!response.ok) {
      const body = await response.text().catch(() => "");
      throw new HyperMeshError(
        `HTTP ${response.status}: ${response.statusText}`,
        response.status,
        body,
      );
    }

    return response.json() as Promise<T>;
  }

  async post<T>(path: string, body: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    let response: Response;

    try {
      response = await fetch(url, {
        method: "POST",
        headers: this.buildHeaders({
          Accept: "application/json",
          "Content-Type": "application/json",
        }),
        body: JSON.stringify(body),
      });
    } catch (err) {
      throw new HyperMeshError(
        `Request failed: ${err instanceof Error ? err.message : String(err)}`,
        0,
        "",
      );
    }

    if (!response.ok) {
      const text = await response.text().catch(() => "");
      throw new HyperMeshError(
        `HTTP ${response.status}: ${response.statusText}`,
        response.status,
        text,
      );
    }

    return response.json() as Promise<T>;
  }
}
