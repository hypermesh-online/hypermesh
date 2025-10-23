//! Scheduler error types

use nexus_shared::{NexusError, NodeId, ResourceId};

/// Result type alias for scheduler operations
pub type Result<T> = std::result::Result<T, SchedulerError>;

/// Scheduler-specific error types
#[derive(thiserror::Error, Debug)]
pub enum SchedulerError {
    #[error("Placement error: {message}")]
    Placement { message: String },

    #[error("Autoscaling error: {message}")]
    AutoScaling { message: String },

    #[error("Prediction error: {message}")]
    Prediction { message: String },

    #[error("Optimization error: {message}")]
    Optimization { message: String },

    #[error("Policy violation: {message}")]
    PolicyViolation { message: String },

    #[error("Resource monitoring error: {message}")]
    ResourceMonitoring { message: String },

    #[error("Invalid workload: {message}")]
    InvalidWorkload { message: String },

    #[error("Invalid node: {node_id}")]
    InvalidNode { node_id: NodeId },

    #[error("No available nodes for scheduling")]
    NoAvailableNodes,

    #[error("No suitable nodes for workload: {workload_id}")]
    NoSuitableNodes { workload_id: ResourceId },

    #[error("Insufficient resources: need {required}, available {available}")]
    InsufficientResources { required: String, available: String },

    #[error("Scheduling constraint not satisfied: {constraint}")]
    ConstraintNotSatisfied { constraint: String },

    #[error("Affinity rule violation: {rule}")]
    AffinityViolation { rule: String },

    #[error("Anti-affinity rule violation: {rule}")]
    AntiAffinityViolation { rule: String },

    #[error("Taint toleration not found: {taint}")]
    TaintTolerationMissing { taint: String },

    #[error("Node selector not matched: {selector}")]
    NodeSelectorNotMatched { selector: String },

    #[error("Workload not found: {workload_id}")]
    WorkloadNotFound { workload_id: ResourceId },

    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: NodeId },

    #[error("Scaling limit reached: {limit_type}")]
    ScalingLimitReached { limit_type: String },

    #[error("Runtime error: {message}")]
    RuntimeError { message: String },

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("State management error: {message}")]
    StateError { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("Time error: {0}")]
    Time(#[from] std::time::SystemTimeError),
}

impl SchedulerError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            SchedulerError::NoAvailableNodes => true,
            SchedulerError::InsufficientResources { .. } => true,
            SchedulerError::RuntimeError { .. } => true,
            SchedulerError::NetworkError { .. } => true,
            SchedulerError::StateError { .. } => true,
            SchedulerError::Timeout { .. } => true,
            SchedulerError::Io(_) => true,
            SchedulerError::Join(_) => true,
            _ => false,
        }
    }

    /// Check if the error is related to resource constraints
    pub fn is_resource_constraint(&self) -> bool {
        matches!(
            self,
            SchedulerError::NoAvailableNodes |
            SchedulerError::NoSuitableNodes { .. } |
            SchedulerError::InsufficientResources { .. } |
            SchedulerError::ScalingLimitReached { .. }
        )
    }

    /// Check if the error is related to scheduling policies
    pub fn is_policy_related(&self) -> bool {
        matches!(
            self,
            SchedulerError::PolicyViolation { .. } |
            SchedulerError::ConstraintNotSatisfied { .. } |
            SchedulerError::AffinityViolation { .. } |
            SchedulerError::AntiAffinityViolation { .. } |
            SchedulerError::TaintTolerationMissing { .. } |
            SchedulerError::NodeSelectorNotMatched { .. }
        )
    }

    /// Check if the error is related to configuration
    pub fn is_configuration_error(&self) -> bool {
        matches!(
            self,
            SchedulerError::Configuration { .. } |
            SchedulerError::InvalidWorkload { .. } |
            SchedulerError::InvalidNode { .. }
        )
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            SchedulerError::Placement { .. } => "placement",
            SchedulerError::AutoScaling { .. } => "autoscaling",
            SchedulerError::Prediction { .. } => "prediction",
            SchedulerError::Optimization { .. } => "optimization",
            SchedulerError::PolicyViolation { .. } => "policy",
            SchedulerError::ResourceMonitoring { .. } => "monitoring",
            SchedulerError::InvalidWorkload { .. } => "invalid_workload",
            SchedulerError::InvalidNode { .. } => "invalid_node",
            SchedulerError::NoAvailableNodes => "no_nodes",
            SchedulerError::NoSuitableNodes { .. } => "no_suitable_nodes",
            SchedulerError::InsufficientResources { .. } => "insufficient_resources",
            SchedulerError::ConstraintNotSatisfied { .. } => "constraint_violation",
            SchedulerError::AffinityViolation { .. } => "affinity_violation",
            SchedulerError::AntiAffinityViolation { .. } => "anti_affinity_violation",
            SchedulerError::TaintTolerationMissing { .. } => "taint_toleration",
            SchedulerError::NodeSelectorNotMatched { .. } => "node_selector",
            SchedulerError::WorkloadNotFound { .. } => "workload_not_found",
            SchedulerError::NodeNotFound { .. } => "node_not_found",
            SchedulerError::ScalingLimitReached { .. } => "scaling_limit",
            SchedulerError::RuntimeError { .. } => "runtime",
            SchedulerError::NetworkError { .. } => "network",
            SchedulerError::StateError { .. } => "state",
            SchedulerError::Configuration { .. } => "configuration",
            SchedulerError::Timeout { .. } => "timeout",
            SchedulerError::Serialization(_) => "serialization",
            SchedulerError::Io(_) => "io",
            SchedulerError::Join(_) => "join",
            SchedulerError::Time(_) => "time",
        }
    }

    /// Get severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            SchedulerError::InvalidWorkload { .. } |
            SchedulerError::InvalidNode { .. } |
            SchedulerError::Configuration { .. } => ErrorSeverity::High,
            
            SchedulerError::PolicyViolation { .. } |
            SchedulerError::ConstraintNotSatisfied { .. } |
            SchedulerError::AffinityViolation { .. } |
            SchedulerError::AntiAffinityViolation { .. } => ErrorSeverity::Medium,
            
            SchedulerError::NoAvailableNodes |
            SchedulerError::InsufficientResources { .. } => ErrorSeverity::Medium,
            
            SchedulerError::Timeout { .. } |
            SchedulerError::RuntimeError { .. } => ErrorSeverity::Low,
            
            _ => ErrorSeverity::Low,
        }
    }

    /// Get suggested action for the error
    pub fn suggested_action(&self) -> &'static str {
        match self {
            SchedulerError::NoAvailableNodes => "Add more nodes to the cluster or check node health",
            SchedulerError::InsufficientResources { .. } => "Scale up cluster resources or reduce workload requirements",
            SchedulerError::PolicyViolation { .. } => "Review and adjust scheduling policies",
            SchedulerError::InvalidWorkload { .. } => "Fix workload specification and retry",
            SchedulerError::RuntimeError { .. } => "Check runtime system health and connectivity",
            SchedulerError::NetworkError { .. } => "Verify network connectivity and configuration",
            SchedulerError::Configuration { .. } => "Review scheduler configuration settings",
            _ => "Check system logs for more details",
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium, 
    High,
    Critical,
}

impl From<SchedulerError> for NexusError {
    fn from(err: SchedulerError) -> Self {
        match err {
            SchedulerError::Io(io_err) => NexusError::Network(io_err),
            SchedulerError::Configuration { message } => NexusError::Config(message),
            SchedulerError::Timeout { duration_ms } => NexusError::Timeout { duration_ms },
            other => NexusError::Internal {
                message: other.to_string(),
            },
        }
    }
}

/// Convenience macros for creating scheduler errors
#[macro_export]
macro_rules! placement_error {
    ($msg:expr) => {
        SchedulerError::Placement {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! autoscaling_error {
    ($msg:expr) => {
        SchedulerError::AutoScaling {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! policy_violation {
    ($msg:expr) => {
        SchedulerError::PolicyViolation {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! constraint_not_satisfied {
    ($constraint:expr) => {
        SchedulerError::ConstraintNotSatisfied {
            constraint: $constraint.to_string(),
        }
    };
}

#[macro_export]
macro_rules! insufficient_resources {
    ($required:expr, $available:expr) => {
        SchedulerError::InsufficientResources {
            required: $required.to_string(),
            available: $available.to_string(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_categories() {
        let error = SchedulerError::NoAvailableNodes;
        assert_eq!(error.category(), "no_nodes");
        assert!(error.is_resource_constraint());
        assert!(error.is_retryable());
    }
    
    #[test]
    fn test_error_severity() {
        let config_error = SchedulerError::Configuration { 
            message: "Invalid config".to_string() 
        };
        assert_eq!(config_error.severity(), ErrorSeverity::High);
        
        let timeout_error = SchedulerError::Timeout { duration_ms: 5000 };
        assert_eq!(timeout_error.severity(), ErrorSeverity::Low);
    }
    
    #[test]
    fn test_policy_errors() {
        let policy_error = SchedulerError::PolicyViolation { 
            message: "Resource limit exceeded".to_string() 
        };
        assert!(policy_error.is_policy_related());
        assert_eq!(policy_error.category(), "policy");
    }
    
    #[test]
    fn test_resource_constraint_errors() {
        let resource_error = SchedulerError::InsufficientResources {
            required: "4 cores".to_string(),
            available: "2 cores".to_string(),
        };
        assert!(resource_error.is_resource_constraint());
        assert!(resource_error.is_retryable());
    }
}