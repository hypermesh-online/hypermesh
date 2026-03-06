package hypermesh

import (
	"context"
	"fmt"
)

// BlockchainApi provides access to blockchain endpoints.
type BlockchainApi struct {
	http *httpClient
}

// Height returns the current blockchain height.
func (a *BlockchainApi) Height(ctx context.Context) (*BlockchainHeight, error) {
	var result BlockchainHeight
	if err := a.http.get(ctx, "/api/v1/blockchain/height", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Block returns the block at the given index.
func (a *BlockchainApi) Block(ctx context.Context, index uint64) (*Block, error) {
	var result Block
	path := fmt.Sprintf("/api/v1/blockchain/block/%d", index)
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Validate validates the blockchain integrity and returns the result.
func (a *BlockchainApi) Validate(ctx context.Context) (*ValidationResult, error) {
	var result ValidationResult
	if err := a.http.get(ctx, "/api/v1/blockchain/validate", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
