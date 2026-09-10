pub mod water_path;

pub use water_path::{
    is_cardinal_adjacent, manhattan_distance, PassabilityMode, PathResult, SearchBuffers,
    WaterPathSolver,
};
