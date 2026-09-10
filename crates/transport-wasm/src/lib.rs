use transport_dashboard::{CompanyLedger, TelemetryEngine};
use transport_render::{
    Camera, Color, FrameInterpolator, ProjectionMode, SceneBuilder, SoftwareFramebuffer,
};
use transport_scenario::{MapGenConfig, MapGenerator, ScenarioController, ScenarioDefinition};
use transport_sim::World;
use transport_types::{Ticks, TileIndex};
use transport_world::MapSize;

/// High-level session bridge designed for WebAssembly or embedded canvas hosts.
pub struct WasmGameSession {
    pub world: World,
    pub camera: Camera,
    pub interpolator: FrameInterpolator,
    pub framebuffer: SoftwareFramebuffer,
    pub scenario: ScenarioController,
    pub telemetry: TelemetryEngine,
    pub ledger: CompanyLedger,
}

impl WasmGameSession {
    /// Construct a new session with specified render viewport dimensions and generation seed.
    pub fn new(width: u32, height: u32, seed: u64) -> Self {
        let config = MapGenConfig {
            seed,
            size: MapSize::new(48, 48),
            ..Default::default()
        };

        let world = MapGenerator::generate(&config);
        let mut camera = Camera::new(width as f32, height as f32, ProjectionMode::Isometric);
        camera.offset = transport_render::Vec2::new(
            width as f32 / 2.0,
            (height as f32 / 4.0).max(50.0),
        );

        let interpolator = FrameInterpolator::new();
        let framebuffer = SoftwareFramebuffer::new(width, height);
        let company_id = world.current_company_id;
        let ledger = CompanyLedger::new(company_id, 0);
        let telemetry = TelemetryEngine::new();
        let scenario = ScenarioController::new(ScenarioDefinition::default());

        Self {
            world,
            camera,
            interpolator,
            framebuffer,
            scenario,
            telemetry,
            ledger,
        }
    }

    /// Advance simulation by one discrete tick.
    pub fn tick(&mut self) {
        let next_tick = self.world.tick.0 + 1;
        self.world.tick = Ticks(next_tick);
        self.ledger.tick_advance(next_tick);
        self.telemetry
            .sample_from_world(&self.world, self.world.current_company_id, 100, 50);
        self.scenario.update(&self.world);
    }

    /// Render a frame with sub-tick interpolation factor alpha into the internal framebuffer.
    pub fn render(&mut self, alpha: f32) -> &[u32] {
        let mut scene = SceneBuilder::new();
        scene.build(&self.world, &self.interpolator, &self.camera, alpha);
        scene.render(&mut self.framebuffer, &self.camera, Color::rgba(20, 30, 45, 255));
        &self.framebuffer.pixels
    }

    /// Pan camera by screen pixel offsets.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.camera.pan(transport_render::Vec2::new(dx, dy));
    }

    /// Zoom camera around center.
    pub fn zoom(&mut self, factor: f32) {
        self.camera.set_zoom(self.camera.zoom * factor);
    }

    /// Unproject screen coordinate to world TileIndex.
    pub fn screen_to_tile(&self, sx: f32, sy: f32) -> Option<TileIndex> {
        self.camera
            .screen_to_world(transport_render::Vec2::new(sx, sy))
    }

    /// Raw pointer to pixel buffer (for Wasm linear memory access).
    pub fn pixel_buffer_ptr(&self) -> *const u32 {
        self.framebuffer.pixels.as_ptr()
    }

    /// Total byte length of pixel buffer.
    pub fn pixel_buffer_len(&self) -> usize {
        self.framebuffer.pixels.len() * 4
    }

    /// Export telemetry and financial metrics as JSON.
    pub fn telemetry_json(&self) -> String {
        transport_dashboard::export_json(&self.telemetry, &self.ledger).unwrap_or_default()
    }
}
