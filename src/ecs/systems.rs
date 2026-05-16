use crate::ecs::components::{Animator, Collider, GravityScale, Sprite, Transform, Velocity};
use crate::ecs::resources::{CollisionEvents, Gravity};
use hecs::World;

pub fn animation_system(world: &mut World, dt: f32) {
    for (_, (animator, sprite)) in world.query_mut::<(&mut Animator, &mut Sprite)>() {
        let Some(clip) = animator.current_clip().cloned() else {
            continue;
        };
        animator.elapsed += dt;
        let interval = 1.0 / clip.fps;
        if animator.elapsed >= interval {
            animator.elapsed -= interval;
            let next = animator.frame_index + 1;
            if next < clip.frames.len() {
                animator.frame_index = next;
            } else if clip.looping {
                animator.frame_index = 0;
            }
        }
        if let Some(&frame) = clip.frames.get(animator.frame_index) {
            sprite.frame = frame;
        }
    }
}

pub fn gravity_system(world: &mut World, gravity: &Gravity, dt: f32) {
    for (_, (velocity, scale)) in world.query_mut::<(&mut Velocity, &GravityScale)>() {
        velocity.value += gravity.value * scale.scale * dt;
    }
}

pub fn collision_system(world: &World, events: &mut CollisionEvents) {
    events.clear();

    let entities: Vec<(hecs::Entity, (glam::Vec2, glam::Vec2))> = world
        .query::<(&Transform, &Collider)>()
        .iter()
        .map(|(e, (t, c))| (e, (t.position, c.half_extents)))
        .collect();

    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (ea, (pa, ha)) = entities[i];
            let (eb, (pb, hb)) = entities[j];
            let diff = (pa - pb).abs();
            if diff.x < ha.x + hb.x && diff.y < ha.y + hb.y {
                events.pairs.push((ea, eb));
            }
        }
    }
}

pub fn transform_system(world: &mut World, dt: f32) {
    for (_, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>() {
        transform.position += velocity.value * dt;
    }
}

pub fn resolution_system(world: &mut World) {
    use std::collections::HashSet;

    let dynamic_ids: HashSet<hecs::Entity> =
        world.query::<(&Velocity,)>().iter().map(|(e, _)| e).collect();

    let statics: Vec<(glam::Vec2, glam::Vec2)> = world
        .query::<(&Transform, &Collider)>()
        .iter()
        .filter(|(e, _)| !dynamic_ids.contains(e))
        .map(|(_, (t, c))| (t.position, c.half_extents))
        .collect();

    if statics.is_empty() {
        return;
    }

    for (_, (transform, velocity, collider)) in
        world.query_mut::<(&mut Transform, &mut Velocity, &Collider)>()
    {
        for &(spos, she) in &statics {
            let diff = transform.position - spos;
            let overlap_x = (collider.half_extents.x + she.x) - diff.x.abs();
            let overlap_y = (collider.half_extents.y + she.y) - diff.y.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                if overlap_x < overlap_y {
                    transform.position.x += diff.x.signum() * overlap_x;
                    velocity.value.x = 0.0;
                } else {
                    transform.position.y += diff.y.signum() * overlap_y;
                    velocity.value.y = 0.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetServer;
    use crate::ecs::components::*;
    use hecs::World;

    fn make_sprite(server: &mut AssetServer) -> Sprite {
        let sheet = SpriteSheet {
            frame_width: 32,
            frame_height: 32,
            columns: 4,
            rows: 4,
        };
        Sprite {
            sheet: server.load_sheet("test.png", sheet),
            frame: 0,
            color: [1.0; 4],
            flip_x: false,
            flip_y: false,
        }
    }

    fn make_run_animator() -> Animator {
        let mut anim = Animator::new();
        anim.add_clip(
            "run",
            AnimationClip {
                frames: vec![0, 1, 2],
                fps: 10.0,
                looping: true,
            },
        );
        anim.add_clip(
            "die",
            AnimationClip {
                frames: vec![3, 4],
                fps: 10.0,
                looping: false,
            },
        );
        anim.play("run");
        anim
    }

    #[test]
    fn animation_advances_frame_after_interval() {
        let mut world = World::new();
        let mut server = AssetServer::new();
        let entity = world.spawn((make_sprite(&mut server), make_run_animator()));
        animation_system(&mut world, 0.11);
        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 1);
    }

    #[test]
    fn animation_loops_back_to_first_frame() {
        let mut world = World::new();
        let mut server = AssetServer::new();
        let entity = world.spawn((make_sprite(&mut server), make_run_animator()));
        animation_system(&mut world, 0.11); // → frame 1
        animation_system(&mut world, 0.11); // → frame 2
        animation_system(&mut world, 0.11); // → wraps to frame 0
        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 0);
    }

    #[test]
    fn non_looping_animation_clamps_at_last_frame() {
        let mut world = World::new();
        let mut server = AssetServer::new();
        let mut animator = make_run_animator();
        animator.play("die"); // frames: [3, 4], non-looping
        let entity = world.spawn((make_sprite(&mut server), animator));
        animation_system(&mut world, 0.11); // → frame 4
        animation_system(&mut world, 0.11); // stays on 4
        animation_system(&mut world, 0.11);
        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 4);
    }

    #[test]
    fn gravity_accelerates_velocity_downward() {
        let mut world = World::new();
        let gravity = Gravity::default(); // (0, -980)
        let entity = world.spawn((
            Velocity {
                value: glam::Vec2::ZERO,
            },
            GravityScale { scale: 1.0 },
        ));
        gravity_system(&mut world, &gravity, 1.0);
        let vel = world.get::<&Velocity>(entity).unwrap();
        assert!((vel.value.y - (-980.0)).abs() < 0.01);
        assert_eq!(vel.value.x, 0.0);
    }

    #[test]
    fn gravity_scale_zero_is_unaffected() {
        let mut world = World::new();
        let gravity = Gravity::default();
        let entity = world.spawn((
            Velocity {
                value: glam::Vec2::ZERO,
            },
            GravityScale { scale: 0.0 },
        ));
        gravity_system(&mut world, &gravity, 1.0);
        let vel = world.get::<&Velocity>(entity).unwrap();
        assert_eq!(vel.value, glam::Vec2::ZERO);
    }

    #[test]
    fn entities_without_gravity_scale_are_unaffected() {
        let mut world = World::new();
        let gravity = Gravity::default();
        let entity = world.spawn((Velocity {
            value: glam::Vec2::ZERO,
        },));
        gravity_system(&mut world, &gravity, 1.0);
        let vel = world.get::<&Velocity>(entity).unwrap();
        assert_eq!(vel.value, glam::Vec2::ZERO);
    }

    #[test]
    fn overlapping_aabbs_generate_collision_event() {
        let mut world = World::new();
        let a = world.spawn((
            Transform {
                position: glam::Vec2::new(0.0, 0.0),
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(16.0, 16.0),
            },
        ));
        let b = world.spawn((
            Transform {
                position: glam::Vec2::new(10.0, 0.0),
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(16.0, 16.0),
            },
        ));
        let mut events = CollisionEvents::default();
        collision_system(&world, &mut events);
        assert_eq!(events.pairs.len(), 1);
        assert!(events.pairs.contains(&(a, b)) || events.pairs.contains(&(b, a)));
    }

    #[test]
    fn non_overlapping_aabbs_produce_no_events() {
        let mut world = World::new();
        world.spawn((
            Transform {
                position: glam::Vec2::ZERO,
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(8.0, 8.0),
            },
        ));
        world.spawn((
            Transform {
                position: glam::Vec2::new(100.0, 0.0),
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(8.0, 8.0),
            },
        ));
        let mut events = CollisionEvents::default();
        collision_system(&world, &mut events);
        assert!(events.pairs.is_empty());
    }

    #[test]
    fn touching_at_exact_edge_is_not_a_collision() {
        let mut world = World::new();
        world.spawn((
            Transform {
                position: glam::Vec2::ZERO,
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(8.0, 8.0),
            },
        ));
        world.spawn((
            Transform {
                position: glam::Vec2::new(16.0, 0.0),
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(8.0, 8.0),
            },
        ));
        let mut events = CollisionEvents::default();
        collision_system(&world, &mut events);
        assert!(events.pairs.is_empty());
    }

    #[test]
    fn transform_system_applies_velocity() {
        let mut world = World::new();
        let entity = world.spawn((
            Transform {
                position: glam::Vec2::ZERO,
                ..Default::default()
            },
            Velocity {
                value: glam::Vec2::new(100.0, 0.0),
            },
        ));
        transform_system(&mut world, 0.5);
        let t = world.get::<&Transform>(entity).unwrap();
        assert!((t.position.x - 50.0).abs() < 0.01);
    }

    // resolution_system tests
    // Ground is at y=450, half_extents (300, 16) → top edge at y=434.
    // Player half_extents (14, 14) → when resting: player.y = 434 - 14 = 420.

    fn spawn_ground(world: &mut World) {
        world.spawn((
            Transform {
                position: glam::Vec2::new(320.0, 450.0),
                ..Default::default()
            },
            Collider {
                half_extents: glam::Vec2::new(300.0, 16.0),
            },
        ));
    }

    #[test]
    fn resolution_pushes_player_out_of_ground() {
        let mut world = World::new();
        spawn_ground(&mut world);
        // Player slightly inside the ground (bottom edge at 436, top of ground at 434)
        let player = world.spawn((
            Transform {
                position: glam::Vec2::new(320.0, 422.0),
                ..Default::default()
            },
            Velocity { value: glam::Vec2::new(0.0, 50.0) },
            Collider { half_extents: glam::Vec2::new(14.0, 14.0) },
        ));
        resolution_system(&mut world);
        let t = world.get::<&Transform>(player).unwrap();
        // Player bottom edge should sit exactly on ground top edge (y=434)
        assert!((t.position.y - 420.0).abs() < 0.01, "expected y≈420, got {}", t.position.y);
    }

    #[test]
    fn resolution_zeroes_y_velocity_on_ground_contact() {
        let mut world = World::new();
        spawn_ground(&mut world);
        let player = world.spawn((
            Transform {
                position: glam::Vec2::new(320.0, 422.0),
                ..Default::default()
            },
            Velocity { value: glam::Vec2::new(10.0, 200.0) },
            Collider { half_extents: glam::Vec2::new(14.0, 14.0) },
        ));
        resolution_system(&mut world);
        let v = world.get::<&Velocity>(player).unwrap();
        assert_eq!(v.value.y, 0.0);
        assert_eq!(v.value.x, 10.0, "x velocity should be unchanged");
    }

    #[test]
    fn resolution_resolves_wall_on_x_axis() {
        let mut world = World::new();
        // Tall wall at x=100, half_extents (10, 200)
        world.spawn((
            Transform {
                position: glam::Vec2::new(100.0, 240.0),
                ..Default::default()
            },
            Collider { half_extents: glam::Vec2::new(10.0, 200.0) },
        ));
        // Player overlapping wall from the right: player left edge at 106, wall right edge at 110
        let player = world.spawn((
            Transform {
                position: glam::Vec2::new(118.0, 240.0),
                ..Default::default()
            },
            Velocity { value: glam::Vec2::new(-100.0, 0.0) },
            Collider { half_extents: glam::Vec2::new(14.0, 14.0) },
        ));
        resolution_system(&mut world);
        let t = world.get::<&Transform>(player).unwrap();
        let v = world.get::<&Velocity>(player).unwrap();
        // Player left edge (t.position.x - 14) should equal wall right edge (110)
        assert!((t.position.x - 124.0).abs() < 0.01, "expected x≈124, got {}", t.position.x);
        assert_eq!(v.value.x, 0.0);
        assert_eq!(v.value.y, 0.0);
    }

    #[test]
    fn resolution_does_not_affect_non_overlapping_entities() {
        let mut world = World::new();
        spawn_ground(&mut world);
        let player = world.spawn((
            Transform {
                position: glam::Vec2::new(320.0, 200.0),
                ..Default::default()
            },
            Velocity { value: glam::Vec2::new(0.0, 50.0) },
            Collider { half_extents: glam::Vec2::new(14.0, 14.0) },
        ));
        resolution_system(&mut world);
        let t = world.get::<&Transform>(player).unwrap();
        assert!((t.position.y - 200.0).abs() < 0.01);
    }

    #[test]
    fn resolution_does_not_move_static_bodies() {
        let mut world = World::new();
        let ground = world.spawn((
            Transform {
                position: glam::Vec2::new(320.0, 450.0),
                ..Default::default()
            },
            Collider { half_extents: glam::Vec2::new(300.0, 16.0) },
        ));
        world.spawn((
            Transform {
                position: glam::Vec2::new(320.0, 422.0),
                ..Default::default()
            },
            Velocity { value: glam::Vec2::new(0.0, 50.0) },
            Collider { half_extents: glam::Vec2::new(14.0, 14.0) },
        ));
        resolution_system(&mut world);
        let t = world.get::<&Transform>(ground).unwrap();
        assert_eq!(t.position.y, 450.0);
    }
}
