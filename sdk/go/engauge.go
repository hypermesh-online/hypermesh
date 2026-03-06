package hypermesh

import "context"

// EngaugeApi provides access to Engauge endpoints.
type EngaugeApi struct {
	http *httpClient
}

// Capacity returns capacity metrics.
func (a *EngaugeApi) Capacity(ctx context.Context) (*EngaugeCapacityMetrics, error) {
	var result EngaugeCapacityMetrics
	if err := a.http.get(ctx, "/api/v1/engauge/capacity", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Traffic returns traffic metrics.
func (a *EngaugeApi) Traffic(ctx context.Context) (*EngaugeTrafficMetrics, error) {
	var result EngaugeTrafficMetrics
	if err := a.http.get(ctx, "/api/v1/engauge/traffic", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// MarketplaceListings returns marketplace listings.
func (a *EngaugeApi) MarketplaceListings(ctx context.Context) (*EngaugeListingList, error) {
	var result EngaugeListingList
	if err := a.http.get(ctx, "/api/v1/engauge/marketplace/listings", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// NodeMetrics returns node metrics.
func (a *EngaugeApi) NodeMetrics(ctx context.Context) (*EngaugeNodeMetrics, error) {
	var result EngaugeNodeMetrics
	if err := a.http.get(ctx, "/api/v1/engauge/node/metrics", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Leases returns active leases.
func (a *EngaugeApi) Leases(ctx context.Context) (*EngaugeLeaseList, error) {
	var result EngaugeLeaseList
	if err := a.http.get(ctx, "/api/v1/engauge/leases", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
