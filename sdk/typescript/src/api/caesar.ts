import type { HttpClient } from "../client.js";
import type {
  CaesarBalance,
  CaesarGovernorParams,
  CaesarRewardInfo,
  CaesarRouteResult,
  CaesarTransactionList,
  CaesarWalletInfo,
} from "../types.js";

export class CaesarApi {
  constructor(private readonly http: HttpClient) {}

  async wallet(): Promise<CaesarWalletInfo> {
    return this.http.get<CaesarWalletInfo>("/api/v1/caesar/wallet");
  }

  async balance(): Promise<CaesarBalance> {
    return this.http.get<CaesarBalance>("/api/v1/caesar/balance");
  }

  async transactions(limit?: number): Promise<CaesarTransactionList> {
    const query = limit !== undefined ? `?limit=${limit}` : "";
    return this.http.get<CaesarTransactionList>(
      `/api/v1/caesar/transactions${query}`,
    );
  }

  async rewards(): Promise<CaesarRewardInfo> {
    return this.http.get<CaesarRewardInfo>("/api/v1/caesar/rewards");
  }

  async routePacket(
    destination: string,
    amountGrams: number,
  ): Promise<CaesarRouteResult> {
    return this.http.post<CaesarRouteResult>("/api/v1/caesar/route", {
      destination,
      amount_grams: amountGrams,
    });
  }

  async governorParams(): Promise<CaesarGovernorParams> {
    return this.http.get<CaesarGovernorParams>(
      "/api/v1/caesar/governor/params",
    );
  }
}
