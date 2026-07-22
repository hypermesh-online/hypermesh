package hypermesh

import "context"

// NGaugeApi provides access to NGauge endpoints.
type NGaugeApi struct {
	http *httpClient
}

// Capacity returns capacity metrics.
func (a *NGaugeApi) Capacity(ctx context.Context) (*NGaugeCapacityMetrics, error) {
	var result NGaugeCapacityMetrics
	if err := a.http.get(ctx, "/api/v1/ngauge/capacity", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Traffic returns traffic metrics.
func (a *NGaugeApi) Traffic(ctx context.Context) (*NGaugeTrafficMetrics, error) {
	var result NGaugeTrafficMetrics
	if err := a.http.get(ctx, "/api/v1/ngauge/traffic", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// MarketplaceListings returns marketplace listings.
func (a *NGaugeApi) MarketplaceListings(ctx context.Context) (*NGaugeListingList, error) {
	var result NGaugeListingList
	if err := a.http.get(ctx, "/api/v1/ngauge/marketplace/listings", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// NodeMetrics returns node metrics.
func (a *NGaugeApi) NodeMetrics(ctx context.Context) (*NGaugeNodeMetrics, error) {
	var result NGaugeNodeMetrics
	if err := a.http.get(ctx, "/api/v1/ngauge/node/metrics", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Leases returns active leases.
func (a *NGaugeApi) Leases(ctx context.Context) (*NGaugeLeaseList, error) {
	var result NGaugeLeaseList
	if err := a.http.get(ctx, "/api/v1/ngauge/leases", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
