package hypermesh

import "context"

// DomainApi provides access to domain-as-network endpoints.
type DomainApi struct {
	http *httpClient
}

// List returns all registered domains.
func (a *DomainApi) List(ctx context.Context) (*DomainList, error) {
	var result DomainList
	if err := a.http.get(ctx, "/api/v1/domain/list", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Register registers a new domain with the given privacy mode.
func (a *DomainApi) Register(ctx context.Context, name, privacy string) error {
	body := DomainRegisterRequest{
		Name:    name,
		Privacy: privacy,
	}
	return a.http.post(ctx, "/api/v1/domain/register", body, nil)
}

// Join joins an existing domain network, optionally with an invitation token.
func (a *DomainApi) Join(ctx context.Context, name, token string) error {
	body := DomainJoinRequest{
		Name:  name,
		Token: token,
	}
	return a.http.post(ctx, "/api/v1/domain/join", body, nil)
}
