package hypermesh

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

// CapabilityTokenHeader — Phase K.2 — header used to ship the
// capability token on HTTP requests to the gateway.
const CapabilityTokenHeader = "X-HyperMesh-Capability"

type httpClient struct {
	baseURL         string
	httpClient      *http.Client
	capabilityToken string // Phase K.2 — base64 of serialized CapabilityToken
}

func newHTTPClient(baseURL string, client *http.Client) *httpClient {
	return &httpClient{
		baseURL:    baseURL,
		httpClient: client,
	}
}

// SetCapabilityToken installs (or rotates) the K.2 capability token.
// Pass an empty string to clear it.
func (h *httpClient) SetCapabilityToken(token string) {
	h.capabilityToken = token
}

// CapabilityToken returns the currently-installed token (or empty).
func (h *httpClient) CapabilityToken() string {
	return h.capabilityToken
}

func (h *httpClient) attachCapabilityHeader(req *http.Request) {
	if h.capabilityToken != "" {
		req.Header.Set(CapabilityTokenHeader, h.capabilityToken)
	}
}

func (h *httpClient) get(ctx context.Context, path string, result any) error {
	url := h.baseURL + path

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("hypermesh: failed to create request: %w", err)
	}
	req.Header.Set("Accept", "application/json")
	h.attachCapabilityHeader(req)

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
	h.attachCapabilityHeader(req)

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
