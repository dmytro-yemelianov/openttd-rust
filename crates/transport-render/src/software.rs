use crate::backend::{Color, RenderBackend, ScreenRect, SpriteId, Vec2};

/// Pure in-memory software rasterizer implementing `RenderBackend`.
/// Allows headless rendering and snapshot generation directly to raw pixel buffers, PPM or BMP images.
#[derive(Debug, Clone)]
pub struct SoftwareFramebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>, // 0xAARRGGBB format
}

impl SoftwareFramebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0xFF000000; (width * height) as usize],
        }
    }

    /// Set a pixel at (x, y) with bounds checking and alpha blending.
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        if color.a == 255 {
            self.pixels[idx] = ((color.a as u32) << 24)
                | ((color.r as u32) << 16)
                | ((color.g as u32) << 8)
                | (color.b as u32);
        } else if color.a > 0 {
            // Alpha blend over existing pixel
            let prev = self.pixels[idx];
            let pr = (prev >> 16) & 0xFF;
            let pg = (prev >> 8) & 0xFF;
            let pb = prev & 0xFF;

            let alpha = color.a as u32;
            let inv_a = 255 - alpha;

            let r = (color.r as u32 * alpha + pr * inv_a) / 255;
            let g = (color.g as u32 * alpha + pg * inv_a) / 255;
            let b = (color.b as u32 * alpha + pb * inv_a) / 255;

            self.pixels[idx] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    /// Draw a filled 2:1 isometric diamond representing a terrain tile.
    pub fn draw_isometric_diamond(&mut self, center_x: i32, center_y: i32, half_w: i32, half_h: i32, color: Color) {
        for dy in -half_h..=half_h {
            let width_at_y = (half_w * (half_h - dy.abs())) / half_h;
            for dx in -width_at_y..=width_at_y {
                self.set_pixel(center_x + dx, center_y + dy, color);
            }
        }
    }

    /// Export the framebuffer as an uncompressed 24-bit BMP image file.
    pub fn to_bmp(&self) -> Vec<u8> {
        let row_padding = (4 - ((self.width * 3) % 4)) % 4;
        let image_size = (self.width * 3 + row_padding) * self.height;
        let file_size = 54 + image_size;

        let mut bmp = Vec::with_capacity(file_size as usize);

        // --- BMP Header (14 bytes) ---
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&file_size.to_le_bytes());
        bmp.extend_from_slice(&[0, 0, 0, 0]); // Reserved
        bmp.extend_from_slice(&54u32.to_le_bytes()); // Pixel array offset

        // --- DIB Header: BITMAPINFOHEADER (40 bytes) ---
        bmp.extend_from_slice(&40u32.to_le_bytes()); // Header size
        bmp.extend_from_slice(&(self.width as i32).to_le_bytes());
        bmp.extend_from_slice(&(self.height as i32).to_le_bytes()); // Bottom-up
        bmp.extend_from_slice(&1u16.to_le_bytes()); // Color planes
        bmp.extend_from_slice(&24u16.to_le_bytes()); // 24-bit RGB
        bmp.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB (uncompressed)
        bmp.extend_from_slice(&image_size.to_le_bytes());
        bmp.extend_from_slice(&2835u32.to_le_bytes()); // 72 DPI X
        bmp.extend_from_slice(&2835u32.to_le_bytes()); // 72 DPI Y
        bmp.extend_from_slice(&0u32.to_le_bytes()); // Total colors
        bmp.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // --- Pixel Data (Bottom-to-Top in standard BMP) ---
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let pixel = self.pixels[(y * self.width + x) as usize];
                let b = (pixel & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let r = ((pixel >> 16) & 0xFF) as u8;
                bmp.push(b);
                bmp.push(g);
                bmp.push(r);
            }
            bmp.resize(bmp.len() + row_padding as usize, 0);
        }

        bmp
    }

    /// Export the framebuffer as a binary Portable Pixmap (PPM P6) byte array.
    pub fn to_ppm(&self) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        let mut out = Vec::with_capacity(header.len() + (self.width * self.height * 3) as usize);
        out.extend_from_slice(header.as_bytes());

        for pixel in &self.pixels {
            let r = ((pixel >> 16) & 0xFF) as u8;
            let g = ((pixel >> 8) & 0xFF) as u8;
            let b = (pixel & 0xFF) as u8;
            out.push(r);
            out.push(g);
            out.push(b);
        }

        out
    }
}

impl RenderBackend for SoftwareFramebuffer {
    fn begin_frame(&mut self, width: u32, height: u32, clear_color: Color) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.pixels.resize((width * height) as usize, 0);
        }

        let clear_val = ((clear_color.a as u32) << 24)
            | ((clear_color.r as u32) << 16)
            | ((clear_color.g as u32) << 8)
            | (clear_color.b as u32);
        self.pixels.fill(clear_val);
    }

    fn draw_sprite(&mut self, pos: Vec2, sprite: SpriteId, _depth: f32, tint: Color) {
        let cx = pos.x as i32;
        let cy = pos.y as i32;

        match sprite {
            SpriteId::Terrain(kind) => {
                let col = match kind {
                    transport_types::enum_::TileKind::Water => Color::rgba(40, 120, 220, 255),
                    transport_types::enum_::TileKind::Grass => Color::rgba(50, 180, 50, 255),
                    transport_types::enum_::TileKind::Clear => Color::rgba(160, 140, 90, 255),
                    transport_types::enum_::TileKind::House => Color::rgba(220, 180, 50, 255),
                    transport_types::enum_::TileKind::Station => Color::rgba(230, 120, 30, 255),
                    transport_types::enum_::TileKind::Trees => Color::rgba(20, 100, 30, 255),
                    _ => Color::rgba(128, 128, 128, 255),
                };
                self.draw_isometric_diamond(cx, cy, 16, 8, col);
            }
            SpriteId::Building(_) => {
                // Draw 3D-ish building block
                self.draw_isometric_diamond(cx, cy, 12, 6, Color::rgba(200, 160, 80, 255));
                for dy in 0..12 {
                    self.set_pixel(cx - 10, cy - dy, Color::rgba(140, 100, 40, 255));
                    self.set_pixel(cx + 10, cy - dy, Color::rgba(180, 130, 60, 255));
                }
            }
            SpriteId::Vehicle(kind, _) => {
                let vcol = match kind {
                    transport_types::enum_::VehicleKind::Ship => Color::rgba(240, 240, 255, 255),
                    _ => Color::rgba(220, 40, 40, 255),
                };
                // Draw 6x6 vehicle marker
                for dy in -3..=3 {
                    for dx in -3..=3 {
                        self.set_pixel(cx + dx, cy + dy, vcol);
                    }
                }
            }
            _ => {
                self.set_pixel(cx, cy, tint);
            }
        }
    }

    fn draw_rect(&mut self, rect: ScreenRect, color: Color, filled: bool) {
        let x0 = rect.x as i32;
        let y0 = rect.y as i32;
        let w = rect.width as i32;
        let h = rect.height as i32;

        if filled {
            for dy in 0..h {
                for dx in 0..w {
                    self.set_pixel(x0 + dx, y0 + dy, color);
                }
            }
        } else {
            for dx in 0..w {
                self.set_pixel(x0 + dx, y0, color);
                self.set_pixel(x0 + dx, y0 + h - 1, color);
            }
            for dy in 0..h {
                self.set_pixel(x0, y0 + dy, color);
                self.set_pixel(x0 + w - 1, y0 + dy, color);
            }
        }
    }

    fn draw_text(&mut self, _pos: Vec2, _text: &str, _size: f32, _color: Color) {
        // Text rasterization stub for software framebuffer
    }

    fn end_frame(&mut self) {
        // Frame finished
    }
}
