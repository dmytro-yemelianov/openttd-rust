use std::cmp::Ordering;
use std::sync::Mutex;
use transport_sim::Command;
use transport_types::{CapabilityToken, CompanyID};

/// Maximum number of client commands admitted per simulation tick.
pub const MAX_INGRESS_PER_TICK: usize = 256;

/// A client command submitted from external actors (UI, network, CLI, AI agents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCommand {
    /// Monotonic sequence number from the client.
    pub sequence: u64,
    /// Target company ID on whose behalf this command is executed.
    pub company_id: CompanyID,
    /// Simulation command payload.
    pub command: Command,
    /// Capability token granting permission for the execution.
    pub capability_token: CapabilityToken,
    /// Internal arrival sequence for stable deterministic tie-breaking.
    pub arrival_id: u64,
}

impl Ord for ClientCommand {
    fn cmp(&self, other: &Self) -> Ordering {
        // Deterministic total ordering:
        // 1. Primary: sequence number
        // 2. Secondary: company_id
        // 3. Tertiary: arrival_id
        self.sequence
            .cmp(&other.sequence)
            .then_with(|| self.company_id.0.cmp(&other.company_id.0))
            .then_with(|| self.arrival_id.cmp(&other.arrival_id))
    }
}

impl PartialOrd for ClientCommand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Thread-safe bounded staging queue for incoming client commands.
pub struct CommandQueue {
    counter: Mutex<u64>,
    pending: Mutex<Vec<ClientCommand>>,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandQueue {
    /// Create a new empty command staging queue.
    pub fn new() -> Self {
        Self {
            counter: Mutex::new(0),
            pending: Mutex::new(Vec::new()),
        }
    }

    /// Submit a command to the staging queue. Returns the allocated arrival ID.
    pub fn submit(
        &self,
        sequence: u64,
        company_id: CompanyID,
        command: Command,
        capability_token: CapabilityToken,
    ) -> u64 {
        let mut counter = self.counter.lock().expect("Command queue counter poisoned");
        let arrival_id = *counter;
        *counter += 1;

        let cmd = ClientCommand {
            sequence,
            company_id,
            command,
            capability_token,
            arrival_id,
        };

        let mut pending = self.pending.lock().expect("Command queue pending poisoned");
        pending.push(cmd);

        arrival_id
    }

    /// Drain up to `MAX_INGRESS_PER_TICK` commands for the upcoming tick,
    /// sorted deterministically by `(sequence, company_id, arrival_id)`.
    pub fn drain_for_tick(&self) -> Vec<ClientCommand> {
        let mut pending = self.pending.lock().expect("Command queue pending poisoned");
        if pending.is_empty() {
            return Vec::new();
        }

        // Sort pending commands to guarantee deterministic order
        pending.sort_unstable();

        let count = std::cmp::min(pending.len(), MAX_INGRESS_PER_TICK);
        let drained: Vec<ClientCommand> = pending.drain(0..count).collect();

        drained
    }

    /// Total count of currently pending commands.
    pub fn pending_count(&self) -> usize {
        self.pending.lock().expect("Pending lock poisoned").len()
    }
}
