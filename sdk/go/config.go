package hypermesh

import (
	"context"
	"fmt"
)

// ConfigApi provides access to configuration endpoints.
type ConfigApi struct {
	http *httpClient
}

// Show returns the full node configuration as a map.
func (a *ConfigApi) Show(ctx context.Context) (map[string]any, error) {
	var result map[string]any
	if err := a.http.get(ctx, "/api/v1/config/show", &result); err != nil {
		return nil, err
	}
	return result, nil
}

// Get returns the value for a specific configuration key.
func (a *ConfigApi) Get(ctx context.Context, key string) (*ConfigValue, error) {
	var result ConfigValue
	path := fmt.Sprintf("/api/v1/config/get/%s", key)
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}
