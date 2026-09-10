use serde::{Deserialize, Serialize};
use transport_sim::World;
use transport_types::CompanyID;

use crate::rrd::FixedRing;

/// High-resolution point-in-time telemetry snapshot of the transport network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub tick: u32,
    pub active_vehicles: usize,
    pub total_cargo_delivered: u64,
    pub total_passengers_served: u64,
    pub average_satisfaction: u8,
    pub treasury_balance: i64,
}

/// Telemetry sampling engine retaining rolling time-series data without allocations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryEngine {
    pub history: FixedRing<TelemetrySnapshot, 300>,
}

impl TelemetryEngine {
    pub fn new() -> Self {
        Self {
            history: FixedRing::new(),
        }
    }

    /// Record an explicit telemetry snapshot into the circular ring.
    pub fn record(&mut self, snapshot: TelemetrySnapshot) {
        self.history.push(snapshot);
    }

    /// Sample telemetry metrics directly from the active simulation World state.
    pub fn sample_from_world(
        &mut self,
        world: &World,
        company_id: CompanyID,
        total_cargo_delivered: u64,
        total_passengers_served: u64,
    ) {
        let active_vehicles = world
            .vehicles
            .values()
            .filter(|v| v.company_id == company_id)
            .count();

        let treasury_balance = world
            .companies
            .get(&company_id)
            .map(|c| c.money.0)
            .unwrap_or(0);

        let average_satisfaction = if world.towns.is_empty() {
            100
        } else {
            let total: u32 = world
                .towns
                .values()
                .map(|t| t.satisfaction_score as u32)
                .sum();
            (total / world.towns.len() as u32) as u8
        };

        let snapshot = TelemetrySnapshot {
            tick: world.tick.0,
            active_vehicles,
            total_cargo_delivered,
            total_passengers_served,
            average_satisfaction,
            treasury_balance,
        };

        self.history.push(snapshot);
    }

    /// Peek the latest snapshot.
    pub fn latest(&self) -> Option<&TelemetrySnapshot> {
        self.history.latest()
    }
}
