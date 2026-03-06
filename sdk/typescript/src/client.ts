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

export class HttpClient {
  private readonly baseUrl: string;

  constructor(baseUrl: string) {
    // Strip trailing slash
    this.baseUrl = baseUrl.replace(/\/+$/, "");
  }

  async get<T>(path: string): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    let response: Response;

    try {
      response = await fetch(url, {
        method: "GET",
        headers: { "Accept": "application/json" },
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
        headers: {
          "Accept": "application/json",
          "Content-Type": "application/json",
        },
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
