package hypermesh

import (
	"context"
	"fmt"
	"net/url"
	"strings"
)

// CatalogApi provides access to Catalog endpoints.
type CatalogApi struct {
	http *httpClient
}

// Browse lists packages with optional query and page parameters.
func (a *CatalogApi) Browse(ctx context.Context, query string, page int) (*CatalogPackageList, error) {
	var parts []string
	if query != "" {
		parts = append(parts, fmt.Sprintf("query=%s", url.QueryEscape(query)))
	}
	if page > 0 {
		parts = append(parts, fmt.Sprintf("page=%d", page))
	}
	path := "/api/v1/catalog/browse"
	if len(parts) > 0 {
		path = fmt.Sprintf("%s?%s", path, strings.Join(parts, "&"))
	}
	var result CatalogPackageList
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Search searches for packages matching the query.
func (a *CatalogApi) Search(ctx context.Context, query string) (*CatalogSearchResults, error) {
	path := fmt.Sprintf("/api/v1/catalog/search?query=%s", url.QueryEscape(query))
	var result CatalogSearchResults
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// PackageInfo returns detailed info about a named package.
func (a *CatalogApi) PackageInfo(ctx context.Context, name string) (*CatalogPackageInfo, error) {
	path := fmt.Sprintf("/api/v1/catalog/package/%s", url.PathEscape(name))
	var result CatalogPackageInfo
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// RegistryStats returns aggregate registry statistics.
func (a *CatalogApi) RegistryStats(ctx context.Context) (*CatalogRegistryStats, error) {
	var result CatalogRegistryStats
	if err := a.http.get(ctx, "/api/v1/catalog/registry/stats", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
