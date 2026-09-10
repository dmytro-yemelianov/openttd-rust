use std::collections::BTreeMap;
use transport_types::{ServiceId, ServicePriority};

use crate::kernel::context::KernelContext;
use crate::kernel::driver::{DriverError, SubsystemDriver};
use crate::kernel::phase::Phase;
use crate::oracle::schema::{OracleTraceFrame, VehicleTrace};
use crate::World;

/// Project a full World state into a canonical compatible Oracle trace frame.
pub fn project_state_slice(world: &World) -> OracleTraceFrame {
    let mut companies = BTreeMap::new();
    for (cid, company) in &world.companies {
        companies.insert(*cid, company.money);
    }

    let mut vehicles = BTreeMap::new();
    for (vid, v) in &world.vehicles {
        vehicles.insert(
            *vid,
            VehicleTrace {
                position: v.position,
                state: v.state,
                cargo: v.cargo.clone(),
            },
        );
    }

    let mut stations = BTreeMap::new();
    for (sid, station) in &world.stations {
        let mut goods_map = BTreeMap::new();
        for g in &station.goods {
            goods_map.insert(g.cargo_type, g.delivered_since_last_visit);
        }
        stations.insert(*sid, goods_map);
    }

    OracleTraceFrame {
        tick: world.tick.0,
        companies,
        vehicles,
        stations,
    }
}

/// Trace recorder driver running during `Phase::Egress` to record canonical state frames.
pub struct TraceRecorder {
    pub frames: Vec<OracleTraceFrame>,
    pub recording: bool,
}

impl TraceRecorder {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            recording: true,
        }
    }

    pub fn record_current_world(&mut self, world: &World) {
        self.frames.push(project_state_slice(world));
    }
}

impl Default for TraceRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubsystemDriver for TraceRecorder {
    fn id(&self) -> ServiceId {
        ServiceId::Custom(99)
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::LOW
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase == Phase::Egress && self.recording {
            self.frames.push(project_state_slice(ctx.world));
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
