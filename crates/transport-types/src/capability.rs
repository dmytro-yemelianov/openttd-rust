use crate::id::CompanyID;
use serde::{Deserialize, Serialize};

/// Capability token representing authorization to perform actions in the microkernel.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CapabilityToken {
    /// Full system authority (kernel internals, privileged drivers)
    System,
    /// Authority restricted to a specific company's resources
    Company(CompanyID),
    /// Read-only observation authority (telemetry, rendering, read-only clients)
    Observer,
}

impl CapabilityToken {
    /// Returns true if this token has authority to mutate resources owned by `company`.
    pub fn can_mutate_company(&self, company: CompanyID) -> bool {
        match self {
            CapabilityToken::System => true,
            CapabilityToken::Company(c) => *c == company,
            CapabilityToken::Observer => false,
        }
    }

    /// Returns true if this token has system-level authority.
    pub fn is_system(&self) -> bool {
        matches!(self, CapabilityToken::System)
    }

    /// Returns true if this token is read-only.
    pub fn is_observer(&self) -> bool {
        matches!(self, CapabilityToken::Observer)
    }
}
