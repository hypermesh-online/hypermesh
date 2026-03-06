package hypermesh

import "context"

// NodeApi provides access to node-level endpoints.
type NodeApi struct {
	http *httpClient
}

// Status returns the current node status.
func (a *NodeApi) Status(ctx context.Context) (*NodeStatus, error) {
	var result NodeStatus
	if err := a.http.get(ctx, "/api/v1/status", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Ping checks if the node is reachable.
func (a *NodeApi) Ping(ctx context.Context) (*PingResponse, error) {
	var result PingResponse
	if err := a.http.get(ctx, "/api/v1/ping", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
