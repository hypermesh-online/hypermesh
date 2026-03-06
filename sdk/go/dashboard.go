package hypermesh

import "context"

// DashboardApi provides access to dashboard endpoints.
type DashboardApi struct {
	http *httpClient
}

// List returns all available dashboards.
func (a *DashboardApi) List(ctx context.Context) (*DashboardList, error) {
	var result DashboardList
	if err := a.http.get(ctx, "/api/v1/dashboard/list", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Info returns information about the dashboard system.
func (a *DashboardApi) Info(ctx context.Context) (*DashboardInfo, error) {
	var result DashboardInfo
	if err := a.http.get(ctx, "/api/v1/dashboard/info", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
