use glam::Vec2;
use hecs::Entity;

#[derive(Debug, Clone)]
pub struct Gravity {
    pub value: Vec2,
}

impl Default for Gravity {
    fn default() -> Self {
        Self { value: Vec2::new(0.0, -980.0) }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Time {
    pub delta: f32,
    pub fixed_delta: f32,
    pub elapsed: f32,
}

#[derive(Debug, Default)]
pub struct CollisionEvents {
    pub pairs: Vec<(Entity, Entity)>,
}

impl CollisionEvents {
    pub fn clear(&mut self) { self.pairs.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_default_points_downward() {
        let g = Gravity::default();
        assert!(g.value.y < 0.0, "gravity should pull downward (negative y)");
        assert_eq!(g.value.x, 0.0);
    }

    #[test]
    fn collision_events_clear_empties_pairs() {
        let mut events = CollisionEvents::default();
        events.pairs.push((hecs::Entity::DANGLING, hecs::Entity::DANGLING));
        events.clear();
        assert!(events.pairs.is_empty());
    }

    #[test]
    fn time_default_is_zero() {
        let t = Time::default();
        assert_eq!(t.elapsed, 0.0);
        assert_eq!(t.delta, 0.0);
    }
}
