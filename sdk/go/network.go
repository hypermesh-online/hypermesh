package hypermesh

import "context"

// NetworkApi provides access to network endpoints.
type NetworkApi struct {
	http *httpClient
}

// Peers returns the list of connected peers.
func (a *NetworkApi) Peers(ctx context.Context) (*PeerList, error) {
	var result PeerList
	if err := a.http.get(ctx, "/api/v1/network/peers", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
