use transport_api::ingress::CommandQueue;
use transport_dashboard::{
    render_telemetry_card, CompanyLedger, FinancialCategory, TelemetryEngine,
};
use transport_render::Camera;
use transport_scenario::{
    MapGenerator, ScenarioController, ScenarioDefinition, ScenarioStatus,
};
use transport_sim::World;
use transport_types::{CompanyID, Ticks, TileIndex};

use crate::terminal_view::render_terminal_viewport;

/// Main application orchestrator binding all simulation, scenario, dashboard and render engines.
pub struct GameApp {
    pub world: World,
    pub scenario: ScenarioController,
    pub telemetry: TelemetryEngine,
    pub ledger: CompanyLedger,
    pub camera: Camera,
    pub cursor: TileIndex,
    pub command_queue: CommandQueue,
    pub company_id: CompanyID,
}

impl GameApp {
    pub fn new(scenario_def: ScenarioDefinition) -> Self {
        let world = MapGenerator::generate(&scenario_def.map_config);
        let company_id = world.current_company_id;
        let ledger = CompanyLedger::new(company_id, world.tick.0);
        let telemetry = TelemetryEngine::new();
        let scenario = ScenarioController::new(scenario_def);

        let camera = Camera::new(800.0, 600.0, transport_render::ProjectionMode::Isometric);
        let cursor = TileIndex::new(world.map.size().width / 2, world.map.size().height / 2);
        let command_queue = CommandQueue::new();

        Self {
            world,
            scenario,
            telemetry,
            ledger,
            camera,
            cursor,
            command_queue,
            company_id,
        }
    }

    /// Advance the simulation by one discrete tick.
    pub fn step_tick(&mut self) {
        let current_tick = self.world.tick.0;
        let next_tick = current_tick + 1;
        self.world.tick = Ticks(next_tick);

        // Advance ledger tick
        self.ledger.tick_advance(next_tick);

        // Periodically earn modest operating revenue or vehicle operating cost
        if next_tick % 30 == 0 && !self.world.vehicles.is_empty() {
            self.ledger
                .record_transaction(FinancialCategory::CargoFreight, 250);
            self.ledger
                .record_transaction(FinancialCategory::VehicleRunning, 50);
            self.scenario.record_cargo_delivered(transport_types::CargoType(1), 5);
        }

        // Sample telemetry
        self.telemetry
            .sample_from_world(&self.world, self.company_id, 120, 45);

        // Update scenario
        self.scenario.update(&self.world);
    }

    /// Run simulation for N ticks sequentially.
    pub fn run_ticks(&mut self, n: u32) {
        for _ in 0..n {
            self.step_tick();
        }
    }

    /// Render a full terminal dashboard screen.
    pub fn render_screen(&self) -> String {
        let mut out = String::new();

        // 1. Telemetry Dashboard & Financial Report
        out.push_str(&render_telemetry_card(&self.telemetry, &self.ledger));
        out.push('\n');

        // 2. Scenario Objectives & Status
        out.push_str("=========================================================================\n");
        out.push_str(&format!(
            " SCENARIO: {} \n",
            self.scenario.definition().title
        ));
        out.push_str(&format!(
            " Briefing: {}\n",
            self.scenario.definition().briefing
        ));

        match self.scenario.status() {
            ScenarioStatus::InProgress {
                elapsed_ticks,
                progress,
            } => {
                out.push_str(&format!(" Status: IN PROGRESS (Tick {elapsed_ticks})\n"));
                for p in progress {
                    let mark = if p.is_satisfied { "✓" } else { "○" };
                    out.push_str(&format!(
                        "   [{}] {} ({}/{})\n",
                        mark, p.description, p.current, p.target
                    ));
                }
            }
            ScenarioStatus::Victory { completed_at_tick } => {
                out.push_str(&format!(
                    " Status: ★★★ VICTORY ACHIEVED at Tick {completed_at_tick} ★★★\n"
                ));
            }
            ScenarioStatus::Defeat { tick, reason } => {
                out.push_str(&format!(
                    " Status: ✕ DEFEAT at Tick {tick}: {reason}\n"
                ));
            }
        }
        out.push_str("=========================================================================\n");

        // 3. Viewport Grid
        out.push_str(&render_terminal_viewport(
            &self.world,
            &self.camera,
            28,
            14,
            Some(self.cursor),
        ));

        out
    }
}
