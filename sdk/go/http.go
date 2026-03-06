package hypermesh

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

type httpClient struct {
	baseURL    string
	httpClient *http.Client
}

func newHTTPClient(baseURL string, client *http.Client) *httpClient {
	return &httpClient{
		baseURL:    baseURL,
		httpClient: client,
	}
}

func (h *httpClient) get(ctx context.Context, path string, result any) error {
	url := h.baseURL + path

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("hypermesh: failed to create request: %w", err)
	}
	req.Header.Set("Accept", "application/json")

	return h.doRequest(req, path, result)
}

func (h *httpClient) post(ctx context.Context, path string, body any, result any) error {
	url := h.baseURL + path

	var reqBody io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("hypermesh: failed to marshal request body: %w", err)
		}
		reqBody = bytes.NewReader(data)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, reqBody)
	if err != nil {
		return fmt.Errorf("hypermesh: failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")

	return h.doRequest(req, path, result)
}

func (h *httpClient) doRequest(req *http.Request, endpoint string, result any) error {
	resp, err := h.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("hypermesh: request failed: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("hypermesh: failed to read response body: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		msg := string(respBody)
		return &HyperMeshError{
			StatusCode: resp.StatusCode,
			Message:    msg,
			Endpoint:   endpoint,
		}
	}

	if result != nil && len(respBody) > 0 {
		if err := json.Unmarshal(respBody, result); err != nil {
			return fmt.Errorf("hypermesh: failed to decode response: %w", err)
		}
	}

	return nil
}
