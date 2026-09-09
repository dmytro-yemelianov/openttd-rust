use thiserror::Error;
use std::fmt;
use crate::id;

/// Errors that can occur in the transport simulation
#[derive(Debug, Error)]
pub enum TransportError {
    /// Invalid ID provided
    #[error("Invalid ID: {0}")]
    InvalidId(String),

    /// Entity not found
    #[error("Entity not found: {0}")]
    NotFound(String),

    /// Operation not allowed in current state
    #[error("Operation not allowed: {0}")]
    InvalidOperation(String),

    /// Insufficient resources (money, cargo, etc.)
    #[error("Insufficient resources: {0}")]
    InsufficientResources(String),

    /// Capacity exceeded
    #[error("Capacity exceeded: {0}")]
    CapacityExceeded(String),

    /// Invalid coordinates or position
    #[error("Invalid position: {0}")]
    InvalidPosition(String),

    /// Path not found
    #[error("No path found: {0}")]
    NoPathFound(String),

    /// Cargo cannot be accepted
    #[error("Cargo not accepted: {0}")]
    CargoNotAccepted(String),

    /// Vehicle cannot move
    #[error("Vehicle cannot move: {0}")]
    CannotMove(String),

    /// Schedule/order error
    #[error("Invalid order: {0}")]
    InvalidOrder(String),

    /// Person-related error
    #[error("Person error: {0}")]
    PersonError(String),

    /// Boarding/authorization error
    #[error("Boarding error: {0}")]
    BoardingError(String),

    /// Simulation invariant violated
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Custom error with context
    #[error("{context}: {source}")]
    Custom {
        context: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Result type for transport simulation operations
pub type Result<T> = std::result::Result<T, TransportError>;

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::InvalidId(id) => write!(f, "Invalid ID: {id}"),
            TransportError::NotFound(entity) => write!(f, "Entity not found: {entity}"),
            TransportError::InvalidOperation(op) => write!(f, "Operation not allowed: {op}"),
            TransportError::InsufficientResources(res) => write!(f, "Insufficient resources: {res}"),
            TransportError::CapacityExceeded(cap) => write!(f, "Capacity exceeded: {cap}"),
            TransportError::InvalidPosition(pos) => write!(f, "Invalid position: {pos}"),
            TransportError::NoPathFound(path) => write!(f, "No path found: {path}"),
            TransportError::CargoNotAccepted(cargo) => write!(f, "Cargo not accepted: {cargo}"),
            TransportError::CannotMove(reason) => write!(f, "Vehicle cannot move: {reason}"),
            TransportError::InvalidOrder(order) => write!(f, "Invalid order: {order}"),
            TransportError::PersonError(err) => write!(f, "Person error: {err}"),
            TransportError::BoardingError(err) => write!(f, "Boarding error: {err}"),
            TransportError::InvariantViolation(inv) => write!(f, "Invariant violation: {inv}"),
            TransportError::ConfigurationError(conf) => write!(f, "Configuration error: {conf}"),
            TransportError::IoError(err) => write!(f, "IO error: {err}"),
            TransportError::SerializationError(err) => write!(f, "Serialization error: {err}"),
            TransportError::Custom { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        TransportError::IoError(err)
    }
}

impl From<serde_json::Error> for TransportError {
    fn from(err: serde_json::Error) -> Self {
        TransportError::SerializationError(err)
    }
}

/// Extension trait for converting results to TransportResult
pub trait TransportResultExt<T, E> {
    fn transport_err(self, context: impl Into<String>) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> TransportResultExt<T, E> for std::result::Result<T, E> {
    fn transport_err(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| TransportError::Custom {
            context: context.into(),
            source: Box::new(e),
        })
    }
}