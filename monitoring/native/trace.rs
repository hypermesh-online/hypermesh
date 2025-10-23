//! Native distributed tracing implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Span represents a unit of work in the distributed system
pub struct Span {
    context: SpanContext,
    operation: String,
    start_time: SystemTime,
    end_time: Option<SystemTime>,
    tags: HashMap<String, String>,
    events: Vec<SpanEvent>,
    status: SpanStatus,
}

/// Span context for propagation across service boundaries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpanContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub trace_flags: TraceFlags,
    pub baggage: HashMap<String, String>,
}

/// Trace ID - unique identifier for a trace
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub [u8; 16]);

impl TraceId {
    /// Generate new random trace ID
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        // Use system time + random for uniqueness
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        bytes[..8].copy_from_slice(&now.as_nanos().to_le_bytes()[..8]);
        // Add some randomness (simplified - would use proper RNG)
        bytes[8..].copy_from_slice(&std::process::id().to_le_bytes()[..4]);
        TraceId(bytes)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Parse from hex string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 16 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(TraceId(arr))
    }
}

/// Span ID - unique identifier for a span
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub [u8; 8]);

impl SpanId {
    /// Generate new random span ID
    pub fn generate() -> Self {
        let mut bytes = [0u8; 8];
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        bytes.copy_from_slice(&now.as_nanos().to_le_bytes()[..8]);
        SpanId(bytes)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Parse from hex string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 8 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes);
        Ok(SpanId(arr))
    }
}

/// Trace flags
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceFlags {
    pub sampled: bool,
    pub debug: bool,
}

impl Default for TraceFlags {
    fn default() -> Self {
        Self {
            sampled: true,
            debug: false,
        }
    }
}

/// Span status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error(String),
}

/// Span event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpanEvent {
    pub timestamp: SystemTime,
    pub name: String,
    pub attributes: HashMap<String, String>,
}

impl Span {
    /// Create new span
    pub fn new(operation: String, parent: Option<SpanContext>) -> Self {
        let (trace_id, parent_span_id) = if let Some(parent) = &parent {
            (parent.trace_id.clone(), Some(parent.span_id.clone()))
        } else {
            (TraceId::generate(), None)
        };

        let context = SpanContext {
            trace_id,
            span_id: SpanId::generate(),
            parent_span_id,
            trace_flags: TraceFlags::default(),
            baggage: HashMap::new(),
        };

        Self {
            context,
            operation,
            start_time: SystemTime::now(),
            end_time: None,
            tags: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        }
    }

    /// Get span context
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// Set tag on span
    pub fn set_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    /// Add event to span
    pub fn add_event(&mut self, name: String, attributes: HashMap<String, String>) {
        self.events.push(SpanEvent {
            timestamp: SystemTime::now(),
            name,
            attributes,
        });
    }

    /// Set span status
    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    /// End the span
    pub fn end(&mut self) {
        self.end_time = Some(SystemTime::now());
    }

    /// Get span duration
    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time).unwrap_or_default())
    }

    /// Export span data
    pub fn export(&self) -> SpanData {
        SpanData {
            trace_id: self.context.trace_id.to_hex(),
            span_id: self.context.span_id.to_hex(),
            parent_span_id: self.context.parent_span_id.as_ref().map(|id| id.to_hex()),
            operation: self.operation.clone(),
            start_time: self.start_time.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
            end_time: self.end_time.map(|t| t.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64),
            duration_ns: self.duration().map(|d| d.as_nanos() as u64),
            tags: self.tags.clone(),
            events: self.events.clone(),
            status: self.status.clone(),
        }
    }
}

/// Exported span data
#[derive(Debug, Serialize, Deserialize)]
pub struct SpanData {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_ns: Option<u64>,
    pub tags: HashMap<String, String>,
    pub events: Vec<SpanEvent>,
    pub status: SpanStatus,
}

/// Tracer for creating and managing spans
pub struct Tracer {
    spans: Arc<RwLock<Vec<SpanData>>>,
    max_spans: usize,
    sampling_rate: f64,
}

impl Tracer {
    /// Create new tracer
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            max_spans: 10000,
            sampling_rate: 1.0,
        }
    }

    /// Create new tracer with configuration
    pub fn with_config(max_spans: usize, sampling_rate: f64) -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            max_spans,
            sampling_rate,
        }
    }

    /// Start new span
    pub fn start_span(&self, operation: &str) -> Span {
        Span::new(operation.to_string(), None)
    }

    /// Start span with parent
    pub fn start_span_with_parent(&self, operation: &str, parent: SpanContext) -> Span {
        Span::new(operation.to_string(), Some(parent))
    }

    /// Record completed span
    pub fn record_span(&self, span: &Span) {
        if self.should_sample() {
            let mut spans = self.spans.write().unwrap();
            spans.push(span.export());

            // Limit stored spans
            if spans.len() > self.max_spans {
                spans.remove(0);
            }
        }
    }

    /// Check if span should be sampled
    fn should_sample(&self) -> bool {
        // Simplified sampling - would use proper random
        self.sampling_rate >= 1.0
    }

    /// Export all spans
    pub fn export(&self) -> Vec<SpanData> {
        self.spans.read().unwrap().clone()
    }

    /// Clear all spans
    pub fn clear(&self) {
        self.spans.write().unwrap().clear();
    }
}

/// Context propagation for distributed tracing
pub struct ContextPropagator;

impl ContextPropagator {
    /// Inject context into headers
    pub fn inject(context: &SpanContext) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("trace-id".to_string(), context.trace_id.to_hex());
        headers.insert("span-id".to_string(), context.span_id.to_hex());
        if let Some(parent) = &context.parent_span_id {
            headers.insert("parent-span-id".to_string(), parent.to_hex());
        }
        headers.insert("trace-flags".to_string(), format!("{:02x}",
            (context.trace_flags.sampled as u8) | ((context.trace_flags.debug as u8) << 1)));

        // Add baggage
        for (key, value) in &context.baggage {
            headers.insert(format!("baggage-{}", key), value.clone());
        }

        headers
    }

    /// Extract context from headers
    pub fn extract(headers: &HashMap<String, String>) -> Option<SpanContext> {
        let trace_id = headers.get("trace-id")
            .and_then(|s| TraceId::from_hex(s).ok())?;
        let span_id = headers.get("span-id")
            .and_then(|s| SpanId::from_hex(s).ok())?;
        let parent_span_id = headers.get("parent-span-id")
            .and_then(|s| SpanId::from_hex(s).ok());

        let trace_flags = headers.get("trace-flags")
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .map(|flags| TraceFlags {
                sampled: (flags & 1) != 0,
                debug: (flags & 2) != 0,
            })
            .unwrap_or_default();

        // Extract baggage
        let mut baggage = HashMap::new();
        for (key, value) in headers {
            if let Some(baggage_key) = key.strip_prefix("baggage-") {
                baggage.insert(baggage_key.to_string(), value.clone());
            }
        }

        Some(SpanContext {
            trace_id,
            span_id,
            parent_span_id,
            trace_flags,
            baggage,
        })
    }
}

// Simplified hex encoding/decoding (would use hex crate in production)
mod hex {
    use std::fmt;

    #[derive(Debug)]
    pub struct FromHexError;

    impl fmt::Display for FromHexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Invalid hex string")
        }
    }

    impl FromHexError {
        pub const InvalidStringLength: Self = FromHexError;
    }

    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, FromHexError> {
        if s.len() % 2 != 0 {
            return Err(FromHexError);
        }

        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| FromHexError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id() {
        let id1 = TraceId::generate();
        let hex = id1.to_hex();
        let id2 = TraceId::from_hex(&hex).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_span_creation() {
        let mut span = Span::new("test_operation".to_string(), None);
        span.set_tag("component".to_string(), "test".to_string());
        span.add_event("test_event".to_string(), HashMap::new());
        span.set_status(SpanStatus::Ok);
        span.end();

        assert!(span.duration().is_some());
        let data = span.export();
        assert_eq!(data.operation, "test_operation");
        assert!(data.end_time.is_some());
    }

    #[test]
    fn test_context_propagation() {
        let context = SpanContext {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_span_id: Some(SpanId::generate()),
            trace_flags: TraceFlags::default(),
            baggage: {
                let mut b = HashMap::new();
                b.insert("user_id".to_string(), "123".to_string());
                b
            },
        };

        let headers = ContextPropagator::inject(&context);
        let extracted = ContextPropagator::extract(&headers).unwrap();

        assert_eq!(context.trace_id, extracted.trace_id);
        assert_eq!(context.span_id, extracted.span_id);
        assert_eq!(context.parent_span_id, extracted.parent_span_id);
        assert_eq!(context.baggage.get("user_id"), extracted.baggage.get("user_id"));
    }

    #[test]
    fn test_tracer() {
        let tracer = Tracer::new();
        let mut span = tracer.start_span("test_op");
        span.end();
        tracer.record_span(&span);

        let spans = tracer.export();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].operation, "test_op");
    }
}