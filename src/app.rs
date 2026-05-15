use crate::{
    assets::AssetServer,
    ecs::resources::{CollisionEvents, Gravity, Time},
    input::InputState,
};
use hecs::World;

pub struct AppContext<'a> {
    pub world: &'a mut World,
    pub assets: &'a mut AssetServer,
    pub input: &'a InputState,
    pub gravity: &'a Gravity,
    pub time: &'a Time,
    pub collisions: &'a CollisionEvents,
}

type FixedSystem = Box<dyn Fn(&mut AppContext)>;

pub struct App {
    fixed_systems: Vec<FixedSystem>,
    pub gravity: Gravity,
}

impl App {
    pub fn new() -> Self {
        Self {
            fixed_systems: Vec::new(),
            gravity: Gravity::default(),
        }
    }

    pub fn with_gravity(mut self, gravity: Gravity) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn add_fixed_system<F: Fn(&mut AppContext) + 'static>(&mut self, f: F) -> &mut Self {
        self.fixed_systems.push(Box::new(f));
        self
    }

    pub fn fixed_system_count(&self) -> usize {
        self.fixed_systems.len()
    }

    pub fn tick(
        &self,
        world: &mut World,
        assets: &mut AssetServer,
        input: &InputState,
        collisions: &mut CollisionEvents,
        time: &mut Time,
        dt: f32,
    ) {
        use crate::ecs::systems::{collision_system, gravity_system, transform_system};

        time.fixed_delta = dt;
        time.elapsed += dt;

        gravity_system(world, &self.gravity, dt);

        {
            let ctx_time = time.clone();
            let prev_collisions: &CollisionEvents = &*collisions;
            let mut ctx = AppContext {
                world,
                assets,
                input,
                gravity: &self.gravity,
                time: &ctx_time,
                collisions: prev_collisions,
            };
            for system in &self.fixed_systems {
                system(&mut ctx);
            }
        }

        collision_system(world, collisions);
        transform_system(world, dt);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_counts_registered_systems() {
        let mut app = App::new();
        app.add_fixed_system(|_ctx: &mut AppContext| {});
        app.add_fixed_system(|_ctx: &mut AppContext| {});
        assert_eq!(app.fixed_system_count(), 2);
    }
}
