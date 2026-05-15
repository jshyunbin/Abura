use crate::assets::Handle;
use glam::Vec2;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Velocity {
    pub value: Vec2,
}

#[derive(Debug, Clone)]
pub struct GravityScale {
    pub scale: f32,
}

impl Default for GravityScale {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag(pub u64);

#[derive(Debug, Clone)]
pub struct Collider {
    pub half_extents: Vec2,
}

// Opaque GPU texture marker — actual GpuTexture lives in the renderer
pub struct Texture;

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteSheet {
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}

impl SpriteSheet {
    /// Returns (x, y, width, height) in pixels for frame index `n`.
    pub fn frame_pixel_rect(&self, frame: u32) -> (u32, u32, u32, u32) {
        debug_assert!(self.columns > 0, "SpriteSheet columns must be > 0");
        debug_assert!(
            frame < self.columns * self.rows,
            "frame {} out of bounds for {}x{} grid (max {})",
            frame,
            self.columns,
            self.rows,
            self.columns * self.rows
        );
        let col = frame % self.columns;
        let row = frame / self.columns;
        (
            col * self.frame_width,
            row * self.frame_height,
            self.frame_width,
            self.frame_height,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub sheet: Handle<SpriteSheet>,
    pub frame: u32,
    pub color: [f32; 4], // RGBA tint, [1,1,1,1] = no tint
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub frames: Vec<u32>,
    pub fps: f32,
    pub looping: bool,
}

#[derive(Debug, Default)]
pub struct Animator {
    clips: HashMap<String, AnimationClip>,
    current: String,
    pub frame_index: usize,
    pub elapsed: f32,
}

impl Animator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_clip(&mut self, name: &str, clip: AnimationClip) {
        self.clips.insert(name.to_string(), clip);
    }

    pub fn play(&mut self, name: &str) {
        if self.current != name {
            self.current = name.to_string();
            self.frame_index = 0;
            self.elapsed = 0.0;
        }
    }

    pub fn current_clip(&self) -> Option<&AnimationClip> {
        self.clips.get(&self.current)
    }

    pub fn current_frame(&self) -> u32 {
        self.current_clip()
            .and_then(|c| c.frames.get(self.frame_index))
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_default_is_identity() {
        let t = Transform::default();
        assert_eq!(t.position, glam::Vec2::ZERO);
        assert_eq!(t.scale, glam::Vec2::ONE);
        assert_eq!(t.rotation, 0.0);
    }

    #[test]
    fn tag_equality() {
        assert_eq!(Tag(42), Tag(42));
        assert_ne!(Tag(1), Tag(2));
    }

    #[test]
    fn gravity_scale_default_is_one() {
        assert_eq!(GravityScale::default().scale, 1.0);
    }

    #[test]
    fn spritesheet_frame_uv_first_frame() {
        let sheet = SpriteSheet {
            frame_width: 32,
            frame_height: 32,
            columns: 4,
            rows: 2,
        };
        // frame 0 → col 0, row 0 → pixel origin (0, 0)
        let (x, y, w, h) = sheet.frame_pixel_rect(0);
        assert_eq!((x, y, w, h), (0, 0, 32, 32));
    }

    #[test]
    fn spritesheet_frame_uv_mid_frame() {
        let sheet = SpriteSheet {
            frame_width: 32,
            frame_height: 32,
            columns: 4,
            rows: 2,
        };
        // frame 5 → col = 5%4 = 1, row = 5/4 = 1 → pixel (32, 32)
        let (x, y, w, h) = sheet.frame_pixel_rect(5);
        assert_eq!((x, y, w, h), (32, 32, 32, 32));
    }

    #[test]
    fn animator_play_resets_to_first_frame() {
        let clip = AnimationClip {
            frames: vec![3, 4, 5],
            fps: 10.0,
            looping: true,
        };
        let mut anim = Animator::new();
        anim.add_clip("run", clip);
        anim.play("run");
        assert_eq!(anim.current_frame(), 3);
    }

    #[test]
    fn animator_play_same_clip_does_not_reset() {
        let clip = AnimationClip {
            frames: vec![0, 1, 2],
            fps: 10.0,
            looping: true,
        };
        let mut anim = Animator::new();
        anim.add_clip("run", clip);
        anim.play("run");
        anim.frame_index = 2;
        anim.play("run"); // same clip — should not reset
        assert_eq!(anim.frame_index, 2);
    }
}
