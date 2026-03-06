import type { HttpClient } from "../client.js";
import type {
  TrustChainCertificate,
  TrustChainCertificateList,
  TrustChainDnsZoneList,
  TrustChainRevokeResult,
  TrustChainValidationResult,
} from "../types.js";

export class TrustChainApi {
  constructor(private readonly http: HttpClient) {}

  async certificates(): Promise<TrustChainCertificateList> {
    return this.http.get<TrustChainCertificateList>(
      "/api/v1/trustchain/certificates",
    );
  }

  async issue(
    subject: string,
    scope: string,
  ): Promise<TrustChainCertificate> {
    return this.http.post<TrustChainCertificate>(
      "/api/v1/trustchain/issue",
      { subject, scope },
    );
  }

  async validate(certPem: string): Promise<TrustChainValidationResult> {
    return this.http.post<TrustChainValidationResult>(
      "/api/v1/trustchain/validate",
      { cert_pem: certPem },
    );
  }

  async revoke(certId: string): Promise<TrustChainRevokeResult> {
    return this.http.post<TrustChainRevokeResult>(
      "/api/v1/trustchain/revoke",
      { cert_id: certId },
    );
  }

  async dnsZones(): Promise<TrustChainDnsZoneList> {
    return this.http.get<TrustChainDnsZoneList>(
      "/api/v1/trustchain/dns/zones",
    );
  }
}
