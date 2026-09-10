use super::event::KernelEvent;
use super::intent::KernelIntent;
use crate::World;
use transport_types::{CapabilityToken, Ticks, VehicleID};

/// Capability-gated execution context passed to subsystem drivers during tick phases.
pub struct KernelContext<'a> {
    pub world: &'a mut World,
    pub tick: Ticks,
    pub vehicle_order_buffer: &'a mut Vec<VehicleID>,
    pub intents: &'a mut Vec<(CapabilityToken, KernelIntent)>,
    pub events: &'a mut Vec<KernelEvent>,
}

impl<'a> KernelContext<'a> {
    /// Stage an intent for Phase::Commit validation.
    pub fn stage_intent(&mut self, token: CapabilityToken, intent: KernelIntent) {
        self.intents.push((token, intent));
    }

    /// Emit an event immediately to the egress queue.
    pub fn emit_event(&mut self, event: KernelEvent) {
        self.events.push(event);
    }
}
