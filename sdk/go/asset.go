package hypermesh

import "context"

// AssetApi provides access to asset endpoints.
type AssetApi struct {
	http *httpClient
}

// List returns all registered assets.
func (a *AssetApi) List(ctx context.Context) (*AssetList, error) {
	var result AssetList
	if err := a.http.get(ctx, "/api/v1/asset/list", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
