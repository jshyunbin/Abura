use glam::Vec2;

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
}
