// Package integration defines interfaces for HyperMesh transport layer integration
package integration

import (
	"context"
	"fmt"
	"time"
)

// HyperMeshTransport defines the interface for HyperMesh transport layer
type HyperMeshTransport interface {
	// Connection management
	Connect(config *TransportConfig) (Connection, error)
	Listen(config *ListenerConfig) (Listener, error)
	
	// Transport capabilities
	GetCapabilities() TransportCapabilities
	GetStatistics() TransportStatistics
	
	// Configuration and control
	UpdateConfiguration(config *TransportConfig) error
	Shutdown() error
}

// Connection represents an active HyperMesh connection
type Connection interface {
	// Request execution
	Execute(request *Request) (*Response, error)
	ExecuteAsync(request *Request) (<-chan *Response, <-chan error)
	
	// Stream operations
	CreateStream(streamConfig *StreamConfig) (Stream, error)
	
	// Connection management
	GetRemoteAddress() string
	GetConnectionID() string
	GetConnectionMetrics() ConnectionMetrics
	
	// Health and status
	IsHealthy() bool
	Ping() error
	
	// Lifecycle
	Close() error
}

// Stream represents a bidirectional stream over HyperMesh
type Stream interface {
	// Data operations
	Send(data []byte) error
	Receive() ([]byte, error)
	
	// Stream control
	GetStreamID() int64
	GetStreamMetrics() StreamMetrics
	
	// Lifecycle
	Close() error
}

// Listener accepts incoming HyperMesh connections
type Listener interface {
	// Accept connections
	Accept() (Connection, error)
	AcceptAsync() <-chan Connection
	
	// Listener management
	GetListenAddress() string
	GetListenerMetrics() ListenerMetrics
	
	// Lifecycle
	Close() error
}

// Configuration structures

// TransportConfig configures HyperMesh transport behavior
type TransportConfig struct {
	// Connection details
	Address           string
	Port              int
	Protocol          string
	
	// Performance settings
	EnableMultiplexing bool
	EnableCompression  bool
	CompressionLevel   int
	BufferSize        int
	
	// Timeout settings
	ConnectTimeout    time.Duration
	RequestTimeout    time.Duration
	KeepAliveTimeout  time.Duration
	IdleTimeout       time.Duration
	
	// Security settings
	EnableTLS         bool
	TLSConfig         *TLSConfig
	
	// Advanced settings
	EnableQUIC        bool
	IPv6Only          bool
	CustomHeaders     map[string]string
}

// TLSConfig configures TLS settings
type TLSConfig struct {
	CertificatePath   string
	KeyPath          string
	CACertPath       string
	VerifyPeer       bool
	MinTLSVersion    string
	CipherSuites     []string
}

// ListenerConfig configures HyperMesh listeners
type ListenerConfig struct {
	Address          string
	Port             int
	Protocol         string
	MaxConnections   int
	AcceptTimeout    time.Duration
	TLSConfig       *TLSConfig
}

// StreamConfig configures stream behavior
type StreamConfig struct {
	StreamID         int64
	Priority         Priority
	FlowControlWindow int32
	Timeout          time.Duration
}

// Request and Response structures

// Request represents a HyperMesh request
type Request struct {
	// Request identification
	ID              string
	Method          string
	Path            string
	
	// Headers and metadata
	Headers         map[string]string
	Metadata        map[string]interface{}
	
	// Request body
	Body            []byte
	
	// Request configuration
	Priority        int
	Timeout         time.Duration
	RetryPolicy     *RetryPolicy
	
	// Context
	Context         context.Context
}

// Response represents a HyperMesh response
type Response struct {
	// Response identification
	ID              string
	RequestID       string
	
	// Status information
	StatusCode      int
	StatusMessage   string
	
	// Headers and metadata
	Headers         map[string]string
	Metadata        map[string]interface{}
	
	// Response body
	Body            []byte
	
	// Performance metrics
	Latency         time.Duration
	ProcessingTime  time.Duration
	
	// Transport information
	ConnectionID    string
	StreamID        int64
}

// RetryPolicy defines retry behavior for requests
type RetryPolicy struct {
	MaxAttempts      int
	InitialBackoff   time.Duration
	MaxBackoff       time.Duration
	BackoffMultiplier float64
	RetryableErrors  []string
}

// Priority levels for requests and streams
type Priority int

const (
	PriorityLow Priority = iota
	PriorityNormal
	PriorityHigh
	PriorityCritical
)

// Metrics and statistics structures

// TransportCapabilities describes transport layer capabilities
type TransportCapabilities struct {
	// Protocol support
	SupportedProtocols []string
	MaxConcurrentStreams int64
	MaxConnectionsPerHost int
	
	// Feature support
	SupportsMultiplexing bool
	SupportsCompression  bool
	SupportsEncryption   bool
	SupportsIPv6         bool
	SupportsQUIC         bool
	
	// Performance characteristics
	MaxThroughputMbps   float64
	MinLatencyMicros    int64
	MaxMessageSize      int64
}

// TransportStatistics provides transport layer statistics
type TransportStatistics struct {
	// Connection statistics
	TotalConnections     int64
	ActiveConnections    int64
	FailedConnections    int64
	ConnectionsPerSecond float64
	
	// Request statistics
	TotalRequests        int64
	SuccessfulRequests   int64
	FailedRequests       int64
	RequestsPerSecond    float64
	AverageLatency       time.Duration
	
	// Data transfer statistics
	BytesSent           int64
	BytesReceived       int64
	MessagesPerSecond   float64
	CompressionRatio    float64
	
	// Performance statistics
	P50Latency          time.Duration
	P90Latency          time.Duration
	P99Latency          time.Duration
	ErrorRate           float64
	
	// Resource usage
	MemoryUsageMB       int64
	CPUUsagePercent     float64
	NetworkUtilization  float64
}

// ConnectionMetrics provides per-connection metrics
type ConnectionMetrics struct {
	// Connection info
	ConnectionID        string
	RemoteAddress       string
	EstablishedAt       time.Time
	LastActivity        time.Time
	
	// Request metrics
	TotalRequests       int64
	SuccessfulRequests  int64
	FailedRequests      int64
	AverageLatency      time.Duration
	
	// Data metrics
	BytesSent          int64
	BytesReceived      int64
	
	// Health metrics
	IsHealthy          bool
	LastError          error
	LastHealthCheck    time.Time
}

// StreamMetrics provides per-stream metrics
type StreamMetrics struct {
	// Stream info
	StreamID           int64
	ConnectionID       string
	CreatedAt          time.Time
	LastActivity       time.Time
	
	// Data metrics
	MessagesSent       int64
	MessagesReceived   int64
	BytesSent         int64
	BytesReceived     int64
	
	// Performance metrics
	AverageLatency     time.Duration
	Throughput         float64
	
	// Status
	IsActive          bool
	LastError         error
}

// ListenerMetrics provides listener metrics
type ListenerMetrics struct {
	// Listener info
	ListenAddress      string
	StartedAt          time.Time
	
	// Connection metrics
	TotalAccepted      int64
	ActiveConnections  int64
	RejectedConnections int64
	AcceptRate         float64
	
	// Performance metrics
	AverageAcceptTime  time.Duration
	
	// Status
	IsListening       bool
	LastError         error
}

// Error types for HyperMesh transport

// TransportError represents transport layer errors
type TransportError struct {
	Code       ErrorCode
	Message    string
	Cause      error
	Retryable  bool
	Temporary  bool
}

// ErrorCode defines transport error types
type ErrorCode int

const (
	ErrorCodeUnknown ErrorCode = iota
	ErrorCodeConnectionFailed
	ErrorCodeConnectionTimeout
	ErrorCodeConnectionClosed
	ErrorCodeRequestTimeout
	ErrorCodeRequestTooLarge
	ErrorCodeServerUnavailable
	ErrorCodeTLSError
	ErrorCodeCompressionError
	ErrorCodeProtocolError
	ErrorCodeResourceExhausted
)

// Error implements the error interface
func (te *TransportError) Error() string {
	if te.Cause != nil {
		return fmt.Sprintf("transport error %d: %s (caused by: %v)", te.Code, te.Message, te.Cause)
	}
	return fmt.Sprintf("transport error %d: %s", te.Code, te.Message)
}

// IsRetryable returns whether the error is retryable
func (te *TransportError) IsRetryable() bool {
	return te.Retryable
}

// IsTemporary returns whether the error is temporary
func (te *TransportError) IsTemporary() bool {
	return te.Temporary
}

// Factory functions for creating transport instances

// NewHyperMeshTransport creates a new HyperMesh transport instance
func NewHyperMeshTransport(config *TransportConfig) (HyperMeshTransport, error) {
	// This would be implemented by the actual HyperMesh transport layer
	return &MockHyperMeshTransport{config: config}, nil
}

// MockHyperMeshTransport provides a mock implementation for testing
type MockHyperMeshTransport struct {
	config *TransportConfig
}

// Implement HyperMeshTransport interface for testing
func (m *MockHyperMeshTransport) Connect(config *TransportConfig) (Connection, error) {
	return &MockConnection{
		id:            fmt.Sprintf("conn-%d", time.Now().UnixNano()),
		remoteAddress: config.Address,
		connectedAt:   time.Now(),
	}, nil
}

func (m *MockHyperMeshTransport) Listen(config *ListenerConfig) (Listener, error) {
	return &MockListener{
		address:   config.Address,
		startedAt: time.Now(),
	}, nil
}

func (m *MockHyperMeshTransport) GetCapabilities() TransportCapabilities {
	return TransportCapabilities{
		SupportedProtocols:    []string{"http/2", "quic"},
		MaxConcurrentStreams:  1000,
		MaxConnectionsPerHost: 100,
		SupportsMultiplexing:  true,
		SupportsCompression:   true,
		SupportsEncryption:    true,
		SupportsIPv6:          true,
		SupportsQUIC:          true,
		MaxThroughputMbps:     10000,
		MinLatencyMicros:      100,
		MaxMessageSize:        1024 * 1024, // 1MB
	}
}

func (m *MockHyperMeshTransport) GetStatistics() TransportStatistics {
	return TransportStatistics{
		TotalConnections:   100,
		ActiveConnections:  10,
		FailedConnections:  5,
		TotalRequests:      1000,
		SuccessfulRequests: 950,
		FailedRequests:     50,
		AverageLatency:     150 * time.Microsecond,
		P50Latency:         100 * time.Microsecond,
		P90Latency:         300 * time.Microsecond,
		P99Latency:         500 * time.Microsecond,
	}
}

func (m *MockHyperMeshTransport) UpdateConfiguration(config *TransportConfig) error {
	m.config = config
	return nil
}

func (m *MockHyperMeshTransport) Shutdown() error {
	return nil
}

// MockConnection implements Connection interface for testing
type MockConnection struct {
	id            string
	remoteAddress string
	connectedAt   time.Time
	closed        bool
}

func (m *MockConnection) Execute(request *Request) (*Response, error) {
	if m.closed {
		return nil, &TransportError{
			Code:      ErrorCodeConnectionClosed,
			Message:   "connection is closed",
			Retryable: true,
			Temporary: false,
		}
	}
	
	// Simulate processing time
	processingDelay := 50 * time.Microsecond
	if request.Priority == int(PriorityLow) {
		processingDelay = 200 * time.Microsecond
	}
	time.Sleep(processingDelay)
	
	return &Response{
		ID:             fmt.Sprintf("resp-%d", time.Now().UnixNano()),
		RequestID:      request.ID,
		StatusCode:     200,
		StatusMessage:  "OK",
		Headers:        make(map[string]string),
		Body:           []byte("mock response"),
		Latency:        processingDelay,
		ProcessingTime: processingDelay,
		ConnectionID:   m.id,
	}, nil
}

func (m *MockConnection) ExecuteAsync(request *Request) (<-chan *Response, <-chan error) {
	respChan := make(chan *Response, 1)
	errChan := make(chan error, 1)
	
	go func() {
		resp, err := m.Execute(request)
		if err != nil {
			errChan <- err
		} else {
			respChan <- resp
		}
		close(respChan)
		close(errChan)
	}()
	
	return respChan, errChan
}

func (m *MockConnection) CreateStream(streamConfig *StreamConfig) (Stream, error) {
	return &MockStream{
		id:           streamConfig.StreamID,
		connectionID: m.id,
		createdAt:    time.Now(),
	}, nil
}

func (m *MockConnection) GetRemoteAddress() string {
	return m.remoteAddress
}

func (m *MockConnection) GetConnectionID() string {
	return m.id
}

func (m *MockConnection) GetConnectionMetrics() ConnectionMetrics {
	return ConnectionMetrics{
		ConnectionID:    m.id,
		RemoteAddress:   m.remoteAddress,
		EstablishedAt:   m.connectedAt,
		LastActivity:    time.Now(),
		IsHealthy:       !m.closed,
		LastHealthCheck: time.Now(),
	}
}

func (m *MockConnection) IsHealthy() bool {
	return !m.closed
}

func (m *MockConnection) Ping() error {
	if m.closed {
		return &TransportError{
			Code:    ErrorCodeConnectionClosed,
			Message: "connection is closed",
		}
	}
	return nil
}

func (m *MockConnection) Close() error {
	m.closed = true
	return nil
}

// MockStream implements Stream interface for testing
type MockStream struct {
	id           int64
	connectionID string
	createdAt    time.Time
	closed       bool
}

func (m *MockStream) Send(data []byte) error {
	if m.closed {
		return &TransportError{
			Code:    ErrorCodeConnectionClosed,
			Message: "stream is closed",
		}
	}
	return nil
}

func (m *MockStream) Receive() ([]byte, error) {
	if m.closed {
		return nil, &TransportError{
			Code:    ErrorCodeConnectionClosed,
			Message: "stream is closed",
		}
	}
	return []byte("mock data"), nil
}

func (m *MockStream) GetStreamID() int64 {
	return m.id
}

func (m *MockStream) GetStreamMetrics() StreamMetrics {
	return StreamMetrics{
		StreamID:     m.id,
		ConnectionID: m.connectionID,
		CreatedAt:    m.createdAt,
		LastActivity: time.Now(),
		IsActive:     !m.closed,
	}
}

func (m *MockStream) Close() error {
	m.closed = true
	return nil
}

// MockListener implements Listener interface for testing
type MockListener struct {
	address   string
	startedAt time.Time
	closed    bool
}

func (m *MockListener) Accept() (Connection, error) {
	if m.closed {
		return nil, &TransportError{
			Code:    ErrorCodeConnectionClosed,
			Message: "listener is closed",
		}
	}
	
	return &MockConnection{
		id:            fmt.Sprintf("accepted-%d", time.Now().UnixNano()),
		remoteAddress: "client-address",
		connectedAt:   time.Now(),
	}, nil
}

func (m *MockListener) AcceptAsync() <-chan Connection {
	connChan := make(chan Connection)
	
	go func() {
		for !m.closed {
			conn, err := m.Accept()
			if err == nil {
				connChan <- conn
			}
			time.Sleep(100 * time.Millisecond) // Simulate accept rate
		}
		close(connChan)
	}()
	
	return connChan
}

func (m *MockListener) GetListenAddress() string {
	return m.address
}

func (m *MockListener) GetListenerMetrics() ListenerMetrics {
	return ListenerMetrics{
		ListenAddress: m.address,
		StartedAt:     m.startedAt,
		IsListening:   !m.closed,
	}
}

func (m *MockListener) Close() error {
	m.closed = true
	return nil
}