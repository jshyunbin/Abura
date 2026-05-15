use crate::assets::Handle;
use crate::ecs::components::SpriteSheet;
use glam::Vec2;

pub struct TileMap {
    pub sheet: Handle<SpriteSheet>,
    pub tiles: Vec<u32>,
    pub width: u32,
    pub height: u32,
    pub tile_size: Vec2,
    pub dirty: bool,
}

impl TileMap {
    pub fn new(sheet: Handle<SpriteSheet>, width: u32, height: u32, tile_size: Vec2) -> Self {
        Self {
            sheet,
            tiles: vec![0; (width * height) as usize],
            width,
            height,
            tile_size,
            dirty: true,
        }
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: u32) {
        let idx = (y * self.width + x) as usize;
        if self.tiles[idx] != tile {
            self.tiles[idx] = tile;
            self.dirty = true;
        }
    }
}

/// Normalized UV rect [u0, v0, u1, v1] for a tile in a cols×rows sheet.
pub fn tile_uv(tile: u32, cols: u32, rows: u32) -> [f32; 4] {
    let col = tile % cols;
    let row = tile / cols;
    let fw = 1.0 / cols as f32;
    let fh = 1.0 / rows as f32;
    let u0 = col as f32 * fw;
    let v0 = row as f32 * fh;
    [u0, v0, u0 + fw, v0 + fh]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_uv_first_tile_is_top_left() {
        let uv = tile_uv(0, 4, 2);
        assert!((uv[0] - 0.0).abs() < 1e-6);
        assert!((uv[1] - 0.0).abs() < 1e-6);
        assert!((uv[2] - 0.25).abs() < 1e-6);
        assert!((uv[3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn tile_uv_frame_six_in_4x2_sheet() {
        // tile 6: col = 6%4 = 2, row = 6/4 = 1
        // u0 = 2/4 = 0.5, v0 = 1/2 = 0.5
        let uv = tile_uv(6, 4, 2);
        assert!((uv[0] - 0.5).abs() < 1e-6);
        assert!((uv[1] - 0.5).abs() < 1e-6);
        assert!((uv[2] - 0.75).abs() < 1e-6);
        assert!((uv[3] - 1.0).abs() < 1e-6);
    }
}
