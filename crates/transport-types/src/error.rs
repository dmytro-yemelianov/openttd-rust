use thiserror::Error;

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

    /// Arithmetic overflow occurred
    #[error("Arithmetic overflow: {0}")]
    ArithmeticOverflow(String),

    /// Arithmetic underflow occurred
    #[error("Arithmetic underflow: {0}")]
    Underflow(String),

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

/// Extension trait for converting results to TransportResult
pub trait TransportResultExt<T, E> {
    fn transport_err(self, context: impl Into<String>) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> TransportResultExt<T, E>
    for std::result::Result<T, E>
{
    fn transport_err(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| TransportError::Custom {
            context: context.into(),
            source: Box::new(e),
        })
    }
}
