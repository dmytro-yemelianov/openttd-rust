use super::context::KernelContext;
use super::phase::Phase;
use transport_types::{CapabilityToken, ServiceId, ServicePriority};

/// Error emitted by a subsystem driver during a tick phase.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum DriverError {
    #[error("Unauthorized capability token: {0:?}")]
    Unauthorized(CapabilityToken),
    #[error("Internal driver error: {0}")]
    Internal(String),
    #[error("Solver budget exhausted")]
    BudgetExhausted,
}

/// Pluggable driver interface for subsystem servers in the microkernel.
pub trait SubsystemDriver: Send {
    /// Returns the unique service identifier of this driver.
    fn id(&self) -> ServiceId;

    /// Returns the scheduling priority for this driver (lower numerical value = higher priority).
    fn priority(&self) -> ServicePriority;

    /// Executes the designated phase within the deterministic tick cycle.
    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError>;
}
