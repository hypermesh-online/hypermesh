import type { HttpClient } from "../client.js";
import type { Block, BlockchainHeight, BlockchainValidation } from "../types.js";

export class BlockchainApi {
  constructor(private readonly http: HttpClient) {}

  async height(): Promise<BlockchainHeight> {
    return this.http.get<BlockchainHeight>("/api/v1/blockchain/height");
  }

  async block(index: number): Promise<Block> {
    return this.http.get<Block>(`/api/v1/blockchain/block/${index}`);
  }

  async validate(): Promise<BlockchainValidation> {
    return this.http.get<BlockchainValidation>("/api/v1/blockchain/validate");
  }
}
