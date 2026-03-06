import type { HttpClient } from "../client.js";
import type {
  DomainJoinResponse,
  DomainListResponse,
  DomainRegisterResponse,
} from "../types.js";

export class DomainApi {
  constructor(private readonly http: HttpClient) {}

  async list(): Promise<DomainListResponse> {
    return this.http.get<DomainListResponse>("/api/v1/domain/list");
  }

  async register(
    name: string,
    privacy: string,
  ): Promise<DomainRegisterResponse> {
    return this.http.post<DomainRegisterResponse>("/api/v1/domain/register", {
      name,
      privacy,
    });
  }

  async join(name: string, token?: string): Promise<DomainJoinResponse> {
    return this.http.post<DomainJoinResponse>("/api/v1/domain/join", {
      name,
      ...(token !== undefined ? { token } : {}),
    });
  }
}
