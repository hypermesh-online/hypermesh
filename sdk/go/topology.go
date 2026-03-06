package hypermesh

import "context"

// TopologyApi provides access to topology endpoints.
type TopologyApi struct {
	http *httpClient
}

// Info returns the current topology information.
func (a *TopologyApi) Info(ctx context.Context) (*TopologyInfo, error) {
	var result TopologyInfo
	if err := a.http.get(ctx, "/api/v1/topology/info", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Neighbors returns the neighboring nodes in the matrix topology.
func (a *TopologyApi) Neighbors(ctx context.Context) (*NeighborList, error) {
	var result NeighborList
	if err := a.http.get(ctx, "/api/v1/topology/neighbors", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
