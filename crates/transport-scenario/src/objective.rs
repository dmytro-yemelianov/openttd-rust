use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use transport_sim::World;
use transport_types::unit::Money;
use transport_types::{CargoType, CompanyID, TownID};

use crate::generator::MapGenConfig;

/// Target goal conditions for scenario completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveTarget {
    /// Deliver a specific quantity of cargo.
    CargoDelivered {
        cargo_type: CargoType,
        target_amount: u64,
    },
    /// Safely complete a specific number of passenger commute journeys.
    PassengerTrips { target_trips: u64 },
    /// Grow company treasury to or beyond target capital.
    CompanyNetWorth {
        company_id: CompanyID,
        target_money: Money,
    },
    /// Grow a specific town's population to or beyond target threshold.
    TownPopulation {
        town_id: TownID,
        target_population: u32,
    },
    /// All nested objectives must be met.
    All(Vec<ObjectiveTarget>),
}

/// Progress metrics for an individual objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveProgress {
    pub description: String,
    pub current: u64,
    pub target: u64,
    pub is_satisfied: bool,
}

/// Scenario definition and victory criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    pub id: String,
    pub title: String,
    pub briefing: String,
    #[serde(skip)]
    pub map_config: MapGenConfig,
    pub objectives: Vec<ObjectiveTarget>,
    pub time_limit_ticks: Option<u32>,
}

impl Default for ScenarioDefinition {
    fn default() -> Self {
        Self {
            id: "scen_coastal_commuter".to_string(),
            title: "Coastal Commuter Connection".to_string(),
            briefing: "Establish a reliable maritime ferry link between coastal settlements and achieve 100 passenger trips."
                .to_string(),
            map_config: MapGenConfig::default(),
            objectives: vec![
                ObjectiveTarget::PassengerTrips { target_trips: 100 },
                ObjectiveTarget::CompanyNetWorth {
                    company_id: CompanyID(1),
                    target_money: Money(120_000),
                },
            ],
            time_limit_ticks: Some(10_000),
        }
    }
}

/// Current status of the scenario lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioStatus {
    InProgress {
        elapsed_ticks: u32,
        progress: Vec<ObjectiveProgress>,
    },
    Victory {
        completed_at_tick: u32,
    },
    Defeat {
        tick: u32,
        reason: String,
    },
}

/// Runtime scenario controller that tracks events and evaluates victory/defeat.
pub struct ScenarioController {
    definition: ScenarioDefinition,
    status: ScenarioStatus,
    delivered_cargo: HashMap<CargoType, u64>,
    completed_passenger_trips: u64,
}

impl ScenarioController {
    pub fn new(definition: ScenarioDefinition) -> Self {
        Self {
            definition,
            status: ScenarioStatus::InProgress {
                elapsed_ticks: 0,
                progress: Vec::new(),
            },
            delivered_cargo: HashMap::new(),
            completed_passenger_trips: 0,
        }
    }

    /// Record newly delivered cargo towards scenario objectives.
    pub fn record_cargo_delivered(&mut self, cargo_type: CargoType, amount: u64) {
        *self.delivered_cargo.entry(cargo_type).or_insert(0) += amount;
    }

    /// Record newly completed passenger commute trips towards scenario objectives.
    pub fn record_passenger_trips(&mut self, trips: u64) {
        self.completed_passenger_trips += trips;
    }

    /// Evaluate progress of a single objective target.
    fn evaluate_target(&self, target: &ObjectiveTarget, world: &World) -> ObjectiveProgress {
        match target {
            ObjectiveTarget::CargoDelivered {
                cargo_type,
                target_amount,
            } => {
                let current = self.delivered_cargo.get(cargo_type).copied().unwrap_or(0);
                let is_satisfied = current >= *target_amount;
                ObjectiveProgress {
                    description: format!("Deliver {} units of Cargo {}", target_amount, cargo_type.0),
                    current,
                    target: *target_amount,
                    is_satisfied,
                }
            }
            ObjectiveTarget::PassengerTrips { target_trips } => {
                let current = self.completed_passenger_trips;
                let is_satisfied = current >= *target_trips;
                ObjectiveProgress {
                    description: format!("Complete {target_trips} passenger ferry journeys"),
                    current,
                    target: *target_trips,
                    is_satisfied,
                }
            }
            ObjectiveTarget::CompanyNetWorth {
                company_id,
                target_money,
            } => {
                let current = world
                    .companies
                    .get(company_id)
                    .map(|c| c.money.0.max(0) as u64)
                    .unwrap_or(0);
                let target = target_money.0.max(0) as u64;
                let is_satisfied = current >= target;
                ObjectiveProgress {
                    description: format!("Reach company treasury of ${}", target_money.0),
                    current,
                    target,
                    is_satisfied,
                }
            }
            ObjectiveTarget::TownPopulation {
                town_id,
                target_population,
            } => {
                let current = world
                    .towns
                    .get(town_id)
                    .map(|t| t.census_population as u64)
                    .unwrap_or(0);
                let target = *target_population as u64;
                let is_satisfied = current >= target;
                ObjectiveProgress {
                    description: format!("Grow Town {} to {} citizens", town_id.0, target_population),
                    current,
                    target,
                    is_satisfied,
                }
            }
            ObjectiveTarget::All(children) => {
                let evaluated: Vec<ObjectiveProgress> = children
                    .iter()
                    .map(|c| self.evaluate_target(c, world))
                    .collect();
                let all_met = evaluated.iter().all(|p| p.is_satisfied);
                ObjectiveProgress {
                    description: format!("Complete all {} milestones", children.len()),
                    current: evaluated.iter().filter(|p| p.is_satisfied).count() as u64,
                    target: children.len() as u64,
                    is_satisfied: all_met,
                }
            }
        }
    }

    /// Update scenario state for the current simulation tick.
    pub fn update(&mut self, world: &World) -> &ScenarioStatus {
        if matches!(
            self.status,
            ScenarioStatus::Victory { .. } | ScenarioStatus::Defeat { .. }
        ) {
            return &self.status;
        }

        let tick = world.tick.0;

        // Check time limit
        if let Some(limit) = self.definition.time_limit_ticks {
            if tick >= limit {
                self.status = ScenarioStatus::Defeat {
                    tick,
                    reason: format!("Time limit of {limit} ticks exceeded"),
                };
                return &self.status;
            }
        }

        // Evaluate all objectives
        let progress: Vec<ObjectiveProgress> = self
            .definition
            .objectives
            .iter()
            .map(|obj| self.evaluate_target(obj, world))
            .collect();

        let all_satisfied = progress.iter().all(|p| p.is_satisfied);

        if all_satisfied && !progress.is_empty() {
            self.status = ScenarioStatus::Victory {
                completed_at_tick: tick,
            };
        } else {
            self.status = ScenarioStatus::InProgress {
                elapsed_ticks: tick,
                progress,
            };
        }

        &self.status
    }

    /// Query the scenario definition.
    pub fn definition(&self) -> &ScenarioDefinition {
        &self.definition
    }

    /// Query current status.
    pub fn status(&self) -> &ScenarioStatus {
        &self.status
    }
}
