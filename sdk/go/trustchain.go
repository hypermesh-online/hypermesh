package hypermesh

import "context"

// TrustChainApi provides access to TrustChain endpoints.
type TrustChainApi struct {
	http *httpClient
}

// Certificates returns all certificates.
func (a *TrustChainApi) Certificates(ctx context.Context) (*TrustChainCertificateList, error) {
	var result TrustChainCertificateList
	if err := a.http.get(ctx, "/api/v1/trustchain/certificates", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Issue issues a new certificate for the given subject and scope.
func (a *TrustChainApi) Issue(ctx context.Context, subject, scope string) (*TrustChainCertificate, error) {
	body := TrustChainIssueRequest{Subject: subject, Scope: scope}
	var result TrustChainCertificate
	if err := a.http.post(ctx, "/api/v1/trustchain/issue", body, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Validate validates the given certificate PEM.
func (a *TrustChainApi) Validate(ctx context.Context, certPem string) (*TrustChainValidationResult, error) {
	body := TrustChainValidateRequest{CertPem: certPem}
	var result TrustChainValidationResult
	if err := a.http.post(ctx, "/api/v1/trustchain/validate", body, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Revoke revokes the certificate with the given ID.
func (a *TrustChainApi) Revoke(ctx context.Context, certID string) (*TrustChainRevokeResult, error) {
	body := TrustChainRevokeRequest{CertID: certID}
	var result TrustChainRevokeResult
	if err := a.http.post(ctx, "/api/v1/trustchain/revoke", body, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// DnsZones returns all DNS zones.
func (a *TrustChainApi) DnsZones(ctx context.Context) (*TrustChainDnsZoneList, error) {
	var result TrustChainDnsZoneList
	if err := a.http.get(ctx, "/api/v1/trustchain/dns/zones", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
