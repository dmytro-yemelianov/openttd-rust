use serde::{Deserialize, Serialize};
use transport_types::enum_::{TileKind, VehicleKind};
use transport_types::StationID;
use transport_world::town::BuildingKind;

/// 32-bit RGBA color representation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);
    pub const RED: Self = Self::rgba(230, 50, 50, 255);
    pub const GREEN: Self = Self::rgba(50, 200, 50, 255);
    pub const BLUE: Self = Self::rgba(50, 100, 230, 255);
    pub const YELLOW: Self = Self::rgba(240, 220, 50, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
}

/// 2D continuous screen coordinate point or vector.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub fn distance(&self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn lerp(self, target: Self, alpha: f32) -> Self {
        let a = alpha.clamp(0.0, 1.0);
        Self {
            x: self.x + (target.x - self.x) * a,
            y: self.y + (target.y - self.y) * a,
        }
    }
}

/// 2D screen bounding rectangle.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ScreenRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

/// 8-way directional heading for vehicle and agent rendering.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Heading8 {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Heading8 {
    pub fn from_delta(dx: i32, dy: i32) -> Self {
        match (dx.signum(), dy.signum()) {
            (0, -1) => Self::North,
            (1, -1) => Self::NorthEast,
            (1, 0) => Self::East,
            (1, 1) => Self::SouthEast,
            (0, 1) => Self::South,
            (-1, 1) => Self::SouthWest,
            (-1, 0) => Self::West,
            (-1, -1) => Self::NorthWest,
            _ => Self::North,
        }
    }
}

/// Strongly-typed agnostic sprite identifier resolved by each presentation backend.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpriteId {
    Terrain(TileKind),
    Vehicle(VehicleKind, Heading8),
    Building(BuildingKind),
    Station(StationID),
    Peep,
    WaterSurface,
    Custom(u32),
}

/// Discrete draw command emitted to the render backend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderCommand {
    BeginFrame { width: u32, height: u32, clear: Color },
    DrawSprite { pos: Vec2, sprite: SpriteId, depth: f32, tint: Color },
    DrawRect { rect: ScreenRect, color: Color, filled: bool },
    DrawText { pos: Vec2, text: String, size: f32, color: Color },
    EndFrame,
}

/// Agnostic rendering backend interface.
///
/// Any visual frontend (Terminal ANSI, WGPU, WebGL Canvas, SDL2, etc.)
/// implements this trait to consume the simulation's visual output.
pub trait RenderBackend {
    fn begin_frame(&mut self, width: u32, height: u32, clear_color: Color);
    fn draw_sprite(&mut self, pos: Vec2, sprite: SpriteId, depth: f32, tint: Color);
    fn draw_rect(&mut self, rect: ScreenRect, color: Color, filled: bool);
    fn draw_text(&mut self, pos: Vec2, text: &str, size: f32, color: Color);
    fn end_frame(&mut self);
}

/// Headless, in-memory recording backend for testing and automated validation.
#[derive(Debug, Default, Clone)]
pub struct NullBackend {
    pub commands: Vec<RenderCommand>,
    pub frame_count: u64,
    pub width: u32,
    pub height: u32,
}

impl NullBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sprite_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| matches!(c, RenderCommand::DrawSprite { .. }))
            .count()
    }

    pub fn clear_recorded(&mut self) {
        self.commands.clear();
    }
}

impl RenderBackend for NullBackend {
    fn begin_frame(&mut self, width: u32, height: u32, clear_color: Color) {
        self.width = width;
        self.height = height;
        self.commands.push(RenderCommand::BeginFrame {
            width,
            height,
            clear: clear_color,
        });
    }

    fn draw_sprite(&mut self, pos: Vec2, sprite: SpriteId, depth: f32, tint: Color) {
        self.commands.push(RenderCommand::DrawSprite {
            pos,
            sprite,
            depth,
            tint,
        });
    }

    fn draw_rect(&mut self, rect: ScreenRect, color: Color, filled: bool) {
        self.commands.push(RenderCommand::DrawRect {
            rect,
            color,
            filled,
        });
    }

    fn draw_text(&mut self, pos: Vec2, text: &str, size: f32, color: Color) {
        self.commands.push(RenderCommand::DrawText {
            pos,
            text: text.to_string(),
            size,
            color,
        });
    }

    fn end_frame(&mut self) {
        self.frame_count += 1;
        self.commands.push(RenderCommand::EndFrame);
    }
}
