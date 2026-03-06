package hypermesh

import "fmt"

// HyperMeshError represents an error returned by the HyperMesh API.
type HyperMeshError struct {
	StatusCode int
	Message    string
	Endpoint   string
}

func (e *HyperMeshError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("hypermesh: %s (status %d, endpoint %s)", e.Message, e.StatusCode, e.Endpoint)
	}
	return fmt.Sprintf("hypermesh: HTTP %d from %s", e.StatusCode, e.Endpoint)
}

// IsNotFound returns true if the error is a 404 response.
func (e *HyperMeshError) IsNotFound() bool {
	return e.StatusCode == 404
}

// IsUnauthorized returns true if the error is a 401 response.
func (e *HyperMeshError) IsUnauthorized() bool {
	return e.StatusCode == 401
}
