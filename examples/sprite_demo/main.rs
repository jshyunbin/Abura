use abura::ecs::resources::Gravity;
use abura::input::KeyCode;
use abura::platform::native::{run, NativeApp};
use abura::{
    AnimationClip, Animator, App, AppContext, AssetServer, Collider, GravityScale, Sprite,
    SpriteSheet, Transform, Velocity,
};
use glam::Vec2;
use hecs::World;
use std::collections::HashMap;

fn main() {
    env_logger::init();

    let mut assets = AssetServer::new();
    let sheet = assets.load_sheet(
        "assets/player.png",
        SpriteSheet {
            frame_width: 32,
            frame_height: 32,
            columns: 4,
            rows: 4,
        },
    );

    let mut animator = Animator::new();
    animator.add_clip(
        "idle",
        AnimationClip {
            frames: vec![0],
            fps: 1.0,
            looping: true,
        },
    );
    animator.add_clip(
        "run",
        AnimationClip {
            frames: vec![1, 2, 3],
            fps: 8.0,
            looping: true,
        },
    );
    animator.play("idle");

    let mut world = World::new();
    world.spawn((
        Transform {
            position: Vec2::new(320.0, 240.0),
            scale: Vec2::ONE,
            rotation: 0.0,
        },
        Sprite {
            sheet: sheet.clone(),
            frame: 0,
            color: [1.0; 4],
            flip_x: false,
            flip_y: false,
        },
        animator,
        Velocity::default(),
        GravityScale { scale: 1.0 },
        Collider {
            half_extents: Vec2::new(14.0, 14.0),
        },
    ));

    // Ground platform (static, no gravity)
    world.spawn((
        Transform {
            position: Vec2::new(320.0, 450.0),
            scale: Vec2::ONE,
            rotation: 0.0,
        },
        Collider {
            half_extents: Vec2::new(300.0, 16.0),
        },
    ));

    // The camera uses y-down coordinates (y=0 at top, y increases downward),
    // so gravity must be positive-y to make things fall toward the bottom.
    let mut app = App::new().with_gravity(Gravity { value: Vec2::new(0.0, 980.0) });

    app.add_fixed_system(|ctx: &mut AppContext| {
        for (_, (vel, anim)) in ctx.world.query_mut::<(&mut Velocity, &mut Animator)>() {
            let speed = 200.0;
            vel.value.x = 0.0;
            if ctx.input.keyboard.is_pressed(KeyCode::ArrowLeft) {
                vel.value.x = -speed;
                anim.play("run");
            } else if ctx.input.keyboard.is_pressed(KeyCode::ArrowRight) {
                vel.value.x = speed;
                anim.play("run");
            } else {
                anim.play("idle");
            }
        }
    });

    let mut texture_bytes: HashMap<u64, Vec<u8>> = HashMap::new();
    let sprite_bytes = std::fs::read("examples/sprite_demo/assets/player.png")
        .expect("player.png not found — run Step 18.1 to create it");
    texture_bytes.insert(sheet.id(), sprite_bytes);

    let mut native = NativeApp::new(app, world, assets);
    native.texture_bytes = texture_bytes;

    run(native);
}
