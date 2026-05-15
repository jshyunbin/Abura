use bytemuck::{Pod, Zeroable};
use wgpu::*;

/// Normalized UV rect [u0, v0, u1, v1] for frame `n` in a cols×rows spritesheet.
pub fn uv_rect(frame: u32, cols: u32, rows: u32) -> [f32; 4] {
    let col = frame % cols;
    let row = frame / cols;
    let fw = 1.0 / cols as f32;
    let fh = 1.0 / rows as f32;
    let u0 = col as f32 * fw;
    let v0 = row as f32 * fh;
    [u0, v0, u0 + fw, v0 + fh]
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    pub const ATTRIBS: [VertexAttribute; 3] =
        vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    pub const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteVertex>() as u64,
        step_mode: VertexStepMode::Vertex,
        attributes: &Self::ATTRIBS,
    };
}

/// Builds 4 vertices for a sprite quad centered on `pos` with `size` (w, h).
/// Returns vertices in order: bottom-left, bottom-right, top-right, top-left.
pub fn build_quad(
    pos: [f32; 2],
    size: [f32; 2],
    uv: [f32; 4],
    color: [f32; 4],
    flip_x: bool,
    flip_y: bool,
) -> [SpriteVertex; 4] {
    let hw = size[0] * 0.5;
    let hh = size[1] * 0.5;
    let [u0, v0, u1, v1] = uv;
    let (u0, u1) = if flip_x { (u1, u0) } else { (u0, u1) };
    let (v0, v1) = if flip_y { (v1, v0) } else { (v0, v1) };
    [
        SpriteVertex { position: [pos[0] - hw, pos[1] - hh], uv: [u0, v1], color },
        SpriteVertex { position: [pos[0] + hw, pos[1] - hh], uv: [u1, v1], color },
        SpriteVertex { position: [pos[0] + hw, pos[1] + hh], uv: [u1, v0], color },
        SpriteVertex { position: [pos[0] - hw, pos[1] + hh], uv: [u0, v0], color },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_rect_first_frame_is_top_left() {
        let uv = uv_rect(0, 4, 2);
        assert!((uv[0] - 0.0).abs() < 1e-6);
        assert!((uv[1] - 0.0).abs() < 1e-6);
        assert!((uv[2] - 0.25).abs() < 1e-6);
        assert!((uv[3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn uv_rect_frame_five_in_4x2_sheet() {
        // frame 5: col = 5%4 = 1, row = 5/4 = 1
        let uv = uv_rect(5, 4, 2);
        assert!((uv[0] - 0.25).abs() < 1e-6);
        assert!((uv[1] - 0.5).abs() < 1e-6);
        assert!((uv[2] - 0.5).abs() < 1e-6);
        assert!((uv[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn build_quad_produces_four_vertices() {
        let uv = [0.0_f32, 0.0, 1.0, 1.0];
        let verts = build_quad([0.0, 0.0], [32.0, 32.0], uv, [1.0; 4], false, false);
        assert_eq!(verts.len(), 4);
    }

    #[test]
    fn flip_x_swaps_u_coordinates() {
        let uv = [0.0_f32, 0.0, 0.5, 1.0];
        let normal = build_quad([0.0, 0.0], [32.0, 32.0], uv, [1.0; 4], false, false);
        let flipped = build_quad([0.0, 0.0], [32.0, 32.0], uv, [1.0; 4], true, false);
        assert!((normal[0].uv[0] - 0.0).abs() < 1e-6);
        assert!((flipped[0].uv[0] - 0.5).abs() < 1e-6);
    }
}
