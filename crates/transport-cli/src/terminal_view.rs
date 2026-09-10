use transport_render::Camera;
use transport_sim::World;
use transport_types::enum_::TileKind;
use transport_types::TileIndex;

/// ANSI color constants for clean terminal rendering.
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_CYAN_WATER: &str = "\x1b[36;44m";
const ANSI_GREEN: &str = "\x1b[32;42m";
const ANSI_DARK_GREEN: &str = "\x1b[32;40m";
const ANSI_YELLOW: &str = "\x1b[33;43m";
const ANSI_WHITE_BOLD: &str = "\x1b[1;37;44m";
const ANSI_MAGENTA: &str = "\x1b[1;35;40m";
const ANSI_GRAY: &str = "\x1b[90m";

/// Render a 2D tile-grid viewport of the World state centered on camera view.
pub fn render_terminal_viewport(
    world: &World,
    camera: &Camera,
    view_width: u16,
    view_height: u16,
    cursor: Option<TileIndex>,
) -> String {
    let mut out = String::new();
    let map_size = world.map.size();

    // Calculate tile window bounds centered on camera pan offset
    let cx = (camera.offset.x / camera.tile_width).max(0.0) as u16;
    let cy = (camera.offset.y / camera.tile_height).max(0.0) as u16;

    let start_x = cx.saturating_sub(view_width / 2).min(map_size.width.saturating_sub(view_width));
    let start_y = cy.saturating_sub(view_height / 2).min(map_size.height.saturating_sub(view_height));

    let end_x = (start_x + view_width).min(map_size.width);
    let end_y = (start_y + view_height).min(map_size.height);

    out.push_str(&format!(
        "┌── Viewport [X: {}-{}, Y: {}-{}] ───────────────────────────────────┐\n",
        start_x, end_x.saturating_sub(1), start_y, end_y.saturating_sub(1)
    ));

    for y in start_y..end_y {
        out.push_str("│ ");
        for x in start_x..end_x {
            let tile_idx = TileIndex::new(x, y);

            // Check if cursor is on this tile
            let is_cursor = cursor == Some(tile_idx);

            // Check if vehicle is on this tile
            let has_vehicle = world.vehicles.values().any(|v| v.position == tile_idx);

            if is_cursor {
                out.push_str(&format!("{ANSI_MAGENTA}[] {ANSI_RESET}"));
                continue;
            }

            if has_vehicle {
                out.push_str(&format!("{ANSI_WHITE_BOLD}🚢 {ANSI_RESET}"));
                continue;
            }

            if let Some(tile) = world.map.get(tile_idx) {
                match tile.base.kind {
                    TileKind::Water => {
                        out.push_str(&format!("{ANSI_CYAN_WATER}~~{ANSI_RESET}"));
                    }
                    TileKind::Grass => {
                        out.push_str(&format!("{ANSI_GREEN}..{ANSI_RESET}"));
                    }
                    TileKind::Clear => {
                        out.push_str(&format!("{ANSI_DARK_GREEN}  {ANSI_RESET}"));
                    }
                    TileKind::House => {
                        out.push_str(&format!("{ANSI_YELLOW}⌂ {ANSI_RESET}"));
                    }
                    TileKind::Station => {
                        out.push_str(&format!("{ANSI_YELLOW}⚓ {ANSI_RESET}"));
                    }
                    TileKind::Trees => {
                        out.push_str(&format!("{ANSI_GREEN}♣ {ANSI_RESET}"));
                    }
                    _ => {
                        out.push_str(&format!("{ANSI_GRAY}??{ANSI_RESET}"));
                    }
                }
            } else {
                out.push_str("  ");
            }
        }
        out.push_str(" │\n");
    }

    out.push_str("└──────────────────────────────────────────────────────────────────┘\n");
    out.push_str(&format!(
        " Legend: {ANSI_CYAN_WATER}~~{ANSI_RESET} Water | {ANSI_GREEN}..{ANSI_RESET} Grass | {ANSI_YELLOW}⌂ {ANSI_RESET} Town House | {ANSI_YELLOW}⚓ {ANSI_RESET} Dock | {ANSI_WHITE_BOLD}🚢 {ANSI_RESET} Vessel\n"
    ));
    out
}
