/// The five deterministic execution phases of the microkernel tick cycle.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Phase {
    /// External API commands and ingress buffers are parsed and capability-checked.
    Ingress,
    /// Asynchronous or budgeted graph solvers (e.g. pathfinding) execute their tick step.
    Solvers,
    /// Synchronous subsystem drivers (Movement, Logistics, People) execute deterministic simulation steps.
    Drivers,
    /// Transactional validation and commit of staged intents (e.g. financial ledgers, position commits).
    Commit,
    /// State egress, event notification dispatch, and monotonic clock progression.
    Egress,
}
