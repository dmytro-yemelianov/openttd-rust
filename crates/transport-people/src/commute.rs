use serde::{Deserialize, Serialize};
use transport_types::{BuildingID, Money, Ticks, TileIndex};

/// Fixed origin-to-destination commute plan assigned to an active peep.
///
/// Eliminates OpenTTD passenger laundering: a passenger journey only completes
/// and generates satisfaction when reaching a station adjacent to `destination_tile`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommutePlan {
    pub origin_building: BuildingID,
    pub destination_building: BuildingID,
    pub origin_tile: TileIndex,
    pub destination_tile: TileIndex,
    /// Maximum ticket fare the commuter is willing to pay
    pub max_fare: Money,
    /// Tick when the journey commenced
    pub start_tick: Ticks,
    /// Maximum allowed ticks before the commuter gives up in frustration
    pub max_patience: u32,
    /// Actual cumulative fare deducted so far
    pub total_fare_paid: Money,
}

impl CommutePlan {
    pub fn new(
        origin_building: BuildingID,
        destination_building: BuildingID,
        origin_tile: TileIndex,
        destination_tile: TileIndex,
        max_fare: Money,
        start_tick: Ticks,
        max_patience: u32,
    ) -> Self {
        Self {
            origin_building,
            destination_building,
            origin_tile,
            destination_tile,
            max_fare,
            start_tick,
            max_patience,
            total_fare_paid: Money(0),
        }
    }

    /// Check if the commuter has arrived within walking radius (<= 2 Manhattan distance) of their destination.
    pub fn is_at_destination(&self, current_tile: TileIndex) -> bool {
        let dx = (self.destination_tile.x as i32 - current_tile.x as i32).abs();
        let dy = (self.destination_tile.y as i32 - current_tile.y as i32).abs();
        (dx + dy) <= 2
    }

    /// Calculate commuter satisfaction score upon arrival (0 to 100).
    pub fn calculate_satisfaction(&self, current_tick: Ticks) -> u8 {
        let elapsed = current_tick.0.saturating_sub(self.start_tick.0);
        if elapsed > self.max_patience {
            return 10; // Frustrated / severely delayed
        }

        // Time component: 100 down to 0 based on fraction of patience consumed
        let time_ratio = elapsed as f64 / self.max_patience as f64;
        let time_score = (100.0 * (1.0 - time_ratio.clamp(0.0, 1.0))) as u32;

        // Cost component: satisfaction based on fare paid vs willingness-to-pay
        let cost_score = if self.max_fare.0 > 0 {
            let cost_ratio = self.total_fare_paid.0 as f64 / self.max_fare.0 as f64;
            (100.0 * (1.0 - cost_ratio.clamp(0.0, 1.0))) as u32
        } else {
            100
        };

        // Weighted: 60% travel time + 40% fare
        ((time_score * 6 + cost_score * 4) / 10).clamp(0, 100) as u8
    }
}
