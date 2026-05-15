# Abura Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Abura — a minimal 2D Rust game engine with spritesheet animation, ECS (hecs), gravity, AABB collision, and a single codebase targeting native and WebAssembly.

**Architecture:** Single crate, four internal layers: Platform (winit event loop + input), Renderer (wgpu sprite batching), ECS (hecs World + built-in systems), Engine (App struct + fixed-timestep loop). All platform differences isolated in `src/platform/` — game code never sees `#[cfg(target_arch = "wasm32")]`.

**Tech Stack:** Rust 2021 edition, winit, wgpu, hecs, glam, image, bytemuck, gilrs (native), wasm-bindgen + web-sys (WASM)

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Crate manifest, feature flags (`tilemap`, `audio`), platform-conditional deps |
| `src/lib.rs` | Public re-exports: all types game code needs |
| `src/app.rs` | `App` struct, system registry, `AppContext`, run-loop entry point |
| `src/assets.rs` | `Handle<T>`, `AssetServer` — path deduplication, SpriteSheet metadata storage |
| `src/input.rs` | `InputState`, `KeyboardState`, `MouseState`, `GamepadState` |
| `src/ecs/mod.rs` | Re-exports from ECS submodules |
| `src/ecs/components.rs` | `Transform`, `Sprite`, `SpriteSheet`, `Animator`, `AnimationClip`, `Collider`, `Velocity`, `GravityScale`, `Tag` |
| `src/ecs/resources.rs` | `Gravity`, `Time`, `CollisionEvents` |
| `src/ecs/systems.rs` | `animation_system`, `gravity_system`, `collision_system`, `transform_system` |
| `src/renderer/mod.rs` | `Renderer` — wgpu device/queue/surface init, resize |
| `src/renderer/pipeline.rs` | `SpritePipeline` — render pipeline, bind group layouts, camera uniform |
| `src/renderer/sprite.rs` | `uv_rect()`, `build_quad()`, `SpriteVertex` — pure geometry helpers |
| `src/renderer/texture.rs` | `GpuTexture` — image decode + wgpu texture upload |
| `src/renderer/tilemap.rs` | `TileMap` component + `tile_uv()` helper (feature: `tilemap`) |
| `src/shaders/sprite.wgsl` | WGSL vertex + fragment shader for textured quads |
| `src/platform/mod.rs` | Platform cfg dispatch |
| `src/platform/native.rs` | winit window + event loop + InputState population |
| `src/platform/wasm.rs` | Canvas attachment + async fetch-based asset loading |
| `examples/sprite_demo/main.rs` | End-to-end demo: animated sprite, gravity, keyboard movement |

---

## Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs` and all module stubs

- [ ] **Step 1.1: Create Cargo.toml**

```toml
[package]
name = "abura"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = []
tilemap = []
audio = []

[dependencies]
winit = "0.30"
wgpu = { version = "0.20", features = ["webgl"] }
hecs = "0.10"
glam = "0.28"
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }
bytemuck = { version = "1.16", features = ["derive"] }
log = "0.4"
env_logger = "0.11"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
gilrs = "0.10"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
  "Window", "Document", "HtmlCanvasElement",
  "Request", "Response", "RequestInit", "RequestMode",
] }
console_error_panic_hook = "0.1"
console_log = "0.2"

[dev-dependencies]
approx = "0.5"
```

- [ ] **Step 1.2: Create src/lib.rs**

```rust
pub mod app;
pub mod assets;
pub mod ecs;
pub mod input;
pub mod renderer;
mod platform;

pub use app::{App, AppContext};
pub use assets::{AssetServer, Handle};
pub use ecs::components::*;
pub use ecs::resources::*;
pub use ecs::systems::*;
pub use input::InputState;
```

- [ ] **Step 1.3: Create all module stub files**

Create each file with an empty body (just `// stub`):
- `src/app.rs`
- `src/assets.rs`
- `src/input.rs`
- `src/ecs/mod.rs` — add `pub mod components; pub mod resources; pub mod systems;`
- `src/ecs/components.rs`
- `src/ecs/resources.rs`
- `src/ecs/systems.rs`
- `src/renderer/mod.rs` — add `pub mod pipeline; pub mod sprite; pub mod texture;`
- `src/renderer/pipeline.rs`
- `src/renderer/sprite.rs`
- `src/renderer/texture.rs`
- `src/platform/mod.rs`
- `src/platform/native.rs`
- `src/platform/wasm.rs`
- `src/shaders/sprite.wgsl`
- `examples/sprite_demo/main.rs` — add `fn main() {}`

- [ ] **Step 1.4: Verify compilation**

```bash
cargo check
```

Expected: No errors (empty-file warnings are fine)

- [ ] **Step 1.5: Commit**

```bash
git add Cargo.toml src/ examples/
git commit -m "chore: scaffold Abura crate structure

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Core Component Types

**Files:**
- Modify: `src/ecs/components.rs`

- [ ] **Step 2.1: Write failing tests**

```rust
// At the bottom of src/ecs/components.rs
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
```

- [ ] **Step 2.2: Run tests to verify they fail**

```bash
cargo test ecs::components::tests
```

Expected: FAIL — types not defined

- [ ] **Step 2.3: Implement component types**

```rust
// src/ecs/components.rs
use glam::Vec2;

#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self { position: Vec2::ZERO, scale: Vec2::ONE, rotation: 0.0 }
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
    fn default() -> Self { Self { scale: 1.0 } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag(pub u64);

#[derive(Debug, Clone)]
pub struct Collider {
    pub half_extents: Vec2,
}
```

- [ ] **Step 2.4: Run tests to verify they pass**

```bash
cargo test ecs::components::tests
```

Expected: 3 tests pass

- [ ] **Step 2.5: Commit**

```bash
git add src/ecs/components.rs
git commit -m "feat: add core ECS component types

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Spritesheet & Animator Types

**Files:**
- Modify: `src/ecs/components.rs`
- Modify: `src/assets.rs` (add Handle<T> stub used by Sprite)

- [ ] **Step 3.1: Add Handle<T> stub to assets.rs**

```rust
// src/assets.rs
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) id: u64,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(id: u64) -> Self {
        Self { id, _marker: PhantomData }
    }

    pub fn id(&self) -> u64 { self.id }
}

// Full AssetServer implemented in Task 6
pub struct AssetServer;
```

- [ ] **Step 3.2: Write failing tests**

Append to the `tests` module in `src/ecs/components.rs`:

```rust
    #[test]
    fn spritesheet_frame_uv_first_frame() {
        let sheet = SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 2 };
        // frame 0 → col 0, row 0 → pixel origin (0, 0)
        let (x, y, w, h) = sheet.frame_pixel_rect(0);
        assert_eq!((x, y, w, h), (0, 0, 32, 32));
    }

    #[test]
    fn spritesheet_frame_uv_mid_frame() {
        let sheet = SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 2 };
        // frame 5 → col = 5%4 = 1, row = 5/4 = 1 → pixel (32, 32)
        let (x, y, w, h) = sheet.frame_pixel_rect(5);
        assert_eq!((x, y, w, h), (32, 32, 32, 32));
    }

    #[test]
    fn animator_play_resets_to_first_frame() {
        let clip = AnimationClip { frames: vec![3, 4, 5], fps: 10.0, looping: true };
        let mut anim = Animator::new();
        anim.add_clip("run", clip);
        anim.play("run");
        assert_eq!(anim.current_frame(), 3);
    }

    #[test]
    fn animator_play_same_clip_does_not_reset() {
        let clip = AnimationClip { frames: vec![0, 1, 2], fps: 10.0, looping: true };
        let mut anim = Animator::new();
        anim.add_clip("run", clip);
        anim.play("run");
        anim.frame_index = 2;
        anim.play("run"); // same clip — should not reset
        assert_eq!(anim.frame_index, 2);
    }
```

- [ ] **Step 3.3: Run tests to verify they fail**

```bash
cargo test ecs::components::tests
```

Expected: FAIL — `SpriteSheet`, `Animator`, `AnimationClip` not defined

- [ ] **Step 3.4: Implement Sprite, SpriteSheet, Animator, AnimationClip**

Append to `src/ecs/components.rs`:

```rust
use crate::assets::Handle;
use std::collections::HashMap;

// Opaque GPU texture marker — actual GpuTexture lives in the renderer
pub struct Texture;

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}

impl SpriteSheet {
    /// Returns (x, y, width, height) in pixels for frame index `n`.
    pub fn frame_pixel_rect(&self, frame: u32) -> (u32, u32, u32, u32) {
        let col = frame % self.columns;
        let row = frame / self.columns;
        (col * self.frame_width, row * self.frame_height, self.frame_width, self.frame_height)
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
    pub fn new() -> Self { Self::default() }

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
```

- [ ] **Step 3.5: Run tests to verify they pass**

```bash
cargo test ecs::components::tests
```

Expected: all 7 tests pass

- [ ] **Step 3.6: Commit**

```bash
git add src/ecs/components.rs src/assets.rs
git commit -m "feat: add SpriteSheet, Sprite, AnimationClip, Animator component types

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: ECS Resources

**Files:**
- Modify: `src/ecs/resources.rs`

- [ ] **Step 4.1: Write failing tests**

```rust
// src/ecs/resources.rs
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
```

- [ ] **Step 4.2: Run tests to verify they fail**

```bash
cargo test ecs::resources::tests
```

Expected: FAIL

- [ ] **Step 4.3: Implement resources**

```rust
// src/ecs/resources.rs
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
```

- [ ] **Step 4.4: Run tests to verify they pass**

```bash
cargo test ecs::resources::tests
```

Expected: 3 tests pass

- [ ] **Step 4.5: Commit**

```bash
git add src/ecs/resources.rs
git commit -m "feat: add ECS resources — Gravity, Time, CollisionEvents

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Input State

**Files:**
- Modify: `src/input.rs`

- [ ] **Step 5.1: Write failing tests**

```rust
// src/input.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_pressed_only_fires_on_first_frame() {
        let mut kb = KeyboardState::default();
        kb.press(KeyCode::Space);
        assert!(kb.just_pressed(KeyCode::Space));
        kb.end_frame();
        assert!(!kb.just_pressed(KeyCode::Space));
        assert!(kb.is_pressed(KeyCode::Space));  // still held
    }

    #[test]
    fn just_released_fires_on_release_frame_only() {
        let mut kb = KeyboardState::default();
        kb.press(KeyCode::Space);
        kb.end_frame();
        kb.release(KeyCode::Space);
        assert!(kb.just_released(KeyCode::Space));
        assert!(!kb.is_pressed(KeyCode::Space));
        kb.end_frame();
        assert!(!kb.just_released(KeyCode::Space));
    }

    #[test]
    fn mouse_delta_clears_each_frame() {
        let mut m = MouseState::default();
        m.delta = glam::Vec2::new(5.0, 3.0);
        m.end_frame();
        assert_eq!(m.delta, glam::Vec2::ZERO);
    }
}
```

- [ ] **Step 5.2: Run tests to verify they fail**

```bash
cargo test input::tests
```

Expected: FAIL

- [ ] **Step 5.3: Implement InputState**

```rust
// src/input.rs
use std::collections::HashSet;
use glam::Vec2;

pub use winit::keyboard::KeyCode;
pub use winit::event::MouseButton;

#[derive(Debug, Default)]
pub struct KeyboardState {
    pressed: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
}

impl KeyboardState {
    pub fn press(&mut self, key: KeyCode) {
        if self.pressed.insert(key) {
            self.just_pressed.insert(key);
        }
    }

    pub fn release(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
        self.just_released.insert(key);
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool { self.pressed.contains(&key) }
    pub fn just_pressed(&self, key: KeyCode) -> bool { self.just_pressed.contains(&key) }
    pub fn just_released(&self, key: KeyCode) -> bool { self.just_released.contains(&key) }

    pub fn end_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

#[derive(Debug, Default)]
pub struct MouseState {
    pub position: Vec2,
    pub delta: Vec2,
    pressed: HashSet<MouseButton>,
    just_pressed: HashSet<MouseButton>,
    just_released: HashSet<MouseButton>,
}

impl MouseState {
    pub fn press(&mut self, button: MouseButton) {
        self.just_pressed.insert(button);
        self.pressed.insert(button);
    }

    pub fn release(&mut self, button: MouseButton) {
        self.pressed.remove(&button);
        self.just_released.insert(button);
    }

    pub fn is_pressed(&self, b: MouseButton) -> bool { self.pressed.contains(&b) }
    pub fn just_pressed(&self, b: MouseButton) -> bool { self.just_pressed.contains(&b) }

    pub fn end_frame(&mut self) {
        self.delta = Vec2::ZERO;
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

#[derive(Debug, Default)]
pub struct GamepadState {
    pub buttons: HashSet<u32>,
    pub axes: [f32; 8],
}

impl GamepadState {
    pub fn button_pressed(&self, id: u32) -> bool { self.buttons.contains(&id) }
    pub fn axis_value(&self, id: usize) -> f32 { self.axes.get(id).copied().unwrap_or(0.0) }
}

#[derive(Debug, Default)]
pub struct InputState {
    pub keyboard: KeyboardState,
    pub mouse: MouseState,
    pub gamepad: GamepadState,
}

impl InputState {
    pub fn end_frame(&mut self) {
        self.keyboard.end_frame();
        self.mouse.end_frame();
    }
}
```

- [ ] **Step 5.4: Run tests to verify they pass**

```bash
cargo test input::tests
```

Expected: 3 tests pass

- [ ] **Step 5.5: Commit**

```bash
git add src/input.rs
git commit -m "feat: add InputState with keyboard/mouse/gamepad and just_pressed tracking

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Asset Manager

**Files:**
- Modify: `src/assets.rs`

- [ ] **Step 6.1: Write failing tests**

```rust
// src/assets.rs — append below the Handle<T> stub
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::SpriteSheet;

    fn test_sheet() -> SpriteSheet {
        SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 4 }
    }

    #[test]
    fn same_path_returns_same_handle() {
        let mut server = AssetServer::new();
        let h1 = server.load_sheet("player.png", test_sheet());
        let h2 = server.load_sheet("player.png", test_sheet());
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_paths_return_different_handles() {
        let mut server = AssetServer::new();
        let h1 = server.load_sheet("player.png", test_sheet());
        let h2 = server.load_sheet("enemy.png", test_sheet());
        assert_ne!(h1, h2);
    }

    #[test]
    fn get_sheet_returns_stored_metadata() {
        let mut server = AssetServer::new();
        let handle = server.load_sheet("player.png", test_sheet());
        let sheet = server.get_sheet(&handle).unwrap();
        assert_eq!(sheet.columns, 4);
    }
}
```

- [ ] **Step 6.2: Run tests to verify they fail**

```bash
cargo test assets::tests
```

Expected: FAIL

- [ ] **Step 6.3: Implement AssetServer**

Replace `src/assets.rs` entirely:

```rust
use std::collections::HashMap;
use std::marker::PhantomData;
use crate::ecs::components::SpriteSheet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) id: u64,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(id: u64) -> Self {
        Self { id, _marker: PhantomData }
    }

    pub fn id(&self) -> u64 { self.id }
}

#[derive(Default)]
pub struct AssetServer {
    sheet_paths: HashMap<String, u64>,
    sheets: HashMap<u64, SpriteSheet>,
    next_id: u64,
}

impl AssetServer {
    pub fn new() -> Self { Self::default() }

    /// Register (or retrieve) a SpriteSheet by file path.
    /// Subsequent calls with the same path return the same Handle without
    /// overwriting the stored SpriteSheet.
    pub fn load_sheet(&mut self, path: &str, sheet: SpriteSheet) -> Handle<SpriteSheet> {
        let id = self.sheet_paths.entry(path.to_string()).or_insert_with(|| {
            let id = self.next_id;
            self.next_id += 1;
            id
        });
        self.sheets.entry(*id).or_insert(sheet);
        Handle::new(*id)
    }

    pub fn get_sheet(&self, handle: &Handle<SpriteSheet>) -> Option<&SpriteSheet> {
        self.sheets.get(&handle.id)
    }
}
```

- [ ] **Step 6.4: Run tests to verify they pass**

```bash
cargo test assets::tests
```

Expected: 3 tests pass

- [ ] **Step 6.5: Commit**

```bash
git add src/assets.rs
git commit -m "feat: add AssetServer with SpriteSheet metadata storage and deduplication

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: Animation System (TDD)

**Files:**
- Modify: `src/ecs/systems.rs`

- [ ] **Step 7.1: Write failing tests**

```rust
// src/ecs/systems.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::*;
    use crate::assets::{AssetServer, Handle};
    use hecs::World;

    fn make_sprite(server: &mut AssetServer) -> Sprite {
        let sheet = SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 4 };
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
        anim.add_clip("run", AnimationClip { frames: vec![0, 1, 2], fps: 10.0, looping: true });
        anim.add_clip("die", AnimationClip { frames: vec![3, 4], fps: 10.0, looping: false });
        anim.play("run");
        anim
    }

    #[test]
    fn animation_advances_frame_after_interval() {
        let mut world = World::new();
        let mut server = AssetServer::new();
        let entity = world.spawn((make_sprite(&mut server), make_run_animator()));

        // fps=10 → interval=0.1s; advance by 0.11s should flip to frame index 1
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
        animation_system(&mut world, 0.11); // already at end, should stay on 4
        animation_system(&mut world, 0.11);

        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 4);
    }
}
```

- [ ] **Step 7.2: Run tests to verify they fail**

```bash
cargo test ecs::systems::tests
```

Expected: FAIL — `animation_system` not defined

- [ ] **Step 7.3: Implement animation_system**

```rust
// src/ecs/systems.rs
use hecs::World;
use crate::ecs::components::{Sprite, Animator, Velocity, GravityScale, Transform, Collider};
use crate::ecs::resources::{Gravity, CollisionEvents};

pub fn animation_system(world: &mut World, dt: f32) {
    for (_, (animator, sprite)) in world.query_mut::<(&mut Animator, &mut Sprite)>() {
        let Some(clip) = animator.current_clip().cloned() else { continue };
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
            // non-looping: stay at last frame — no change needed
        }
        sprite.frame = clip.frames[animator.frame_index];
    }
}
```

- [ ] **Step 7.4: Run tests to verify they pass**

```bash
cargo test ecs::systems::tests
```

Expected: 3 tests pass

- [ ] **Step 7.5: Commit**

```bash
git add src/ecs/systems.rs
git commit -m "feat: implement animation_system with looping and clamp-at-end support

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: Gravity System (TDD)

**Files:**
- Modify: `src/ecs/systems.rs`

- [ ] **Step 8.1: Write failing tests**

Append to the `tests` module in `src/ecs/systems.rs`:

```rust
    #[test]
    fn gravity_accelerates_velocity_downward() {
        let mut world = World::new();
        let gravity = Gravity::default(); // (0, -980)
        let entity = world.spawn((
            Velocity { value: glam::Vec2::ZERO },
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
            Velocity { value: glam::Vec2::ZERO },
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
        let entity = world.spawn((Velocity { value: glam::Vec2::ZERO },));

        gravity_system(&mut world, &gravity, 1.0);

        let vel = world.get::<&Velocity>(entity).unwrap();
        assert_eq!(vel.value, glam::Vec2::ZERO);
    }
```

- [ ] **Step 8.2: Run tests to verify they fail**

```bash
cargo test ecs::systems::tests
```

Expected: FAIL on the new 3 tests

- [ ] **Step 8.3: Implement gravity_system**

Append to `src/ecs/systems.rs` (outside the test module):

```rust
pub fn gravity_system(world: &mut World, gravity: &Gravity, dt: f32) {
    for (_, (velocity, scale)) in world.query_mut::<(&mut Velocity, &GravityScale)>() {
        velocity.value += gravity.value * scale.scale * dt;
    }
}
```

- [ ] **Step 8.4: Run tests to verify they pass**

```bash
cargo test ecs::systems::tests
```

Expected: all 6 tests pass

- [ ] **Step 8.5: Commit**

```bash
git add src/ecs/systems.rs
git commit -m "feat: implement gravity_system with per-entity GravityScale

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: AABB Collision & Transform Systems (TDD)

**Files:**
- Modify: `src/ecs/systems.rs`

- [ ] **Step 9.1: Write failing tests**

Append to the `tests` module in `src/ecs/systems.rs`:

```rust
    #[test]
    fn overlapping_aabbs_generate_collision_event() {
        let mut world = World::new();
        let a = world.spawn((
            Transform { position: glam::Vec2::new(0.0, 0.0), ..Default::default() },
            Collider { half_extents: glam::Vec2::new(16.0, 16.0) },
        ));
        let b = world.spawn((
            Transform { position: glam::Vec2::new(10.0, 0.0), ..Default::default() },
            Collider { half_extents: glam::Vec2::new(16.0, 16.0) },
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
            Transform { position: glam::Vec2::ZERO, ..Default::default() },
            Collider { half_extents: glam::Vec2::new(8.0, 8.0) },
        ));
        world.spawn((
            Transform { position: glam::Vec2::new(100.0, 0.0), ..Default::default() },
            Collider { half_extents: glam::Vec2::new(8.0, 8.0) },
        ));
        let mut events = CollisionEvents::default();

        collision_system(&world, &mut events);

        assert!(events.pairs.is_empty());
    }

    #[test]
    fn touching_at_exact_edge_is_not_a_collision() {
        let mut world = World::new();
        // Two 16x16 AABBs (half-extents 8) placed 16 apart — just touching
        world.spawn((
            Transform { position: glam::Vec2::ZERO, ..Default::default() },
            Collider { half_extents: glam::Vec2::new(8.0, 8.0) },
        ));
        world.spawn((
            Transform { position: glam::Vec2::new(16.0, 0.0), ..Default::default() },
            Collider { half_extents: glam::Vec2::new(8.0, 8.0) },
        ));
        let mut events = CollisionEvents::default();

        collision_system(&world, &mut events);

        assert!(events.pairs.is_empty());
    }

    #[test]
    fn transform_system_applies_velocity() {
        let mut world = World::new();
        let entity = world.spawn((
            Transform { position: glam::Vec2::ZERO, ..Default::default() },
            Velocity { value: glam::Vec2::new(100.0, 0.0) },
        ));

        transform_system(&mut world, 0.5); // 0.5s at 100px/s → should move 50px

        let t = world.get::<&Transform>(entity).unwrap();
        assert!((t.position.x - 50.0).abs() < 0.01);
    }
```

- [ ] **Step 9.2: Run tests to verify they fail**

```bash
cargo test ecs::systems::tests
```

Expected: FAIL on the new 4 tests

- [ ] **Step 9.3: Implement collision_system and transform_system**

Append to `src/ecs/systems.rs`:

```rust
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
```

- [ ] **Step 9.4: Run all tests to verify they pass**

```bash
cargo test
```

Expected: all tests pass (10+ tests across all modules)

- [ ] **Step 9.5: Commit**

```bash
git add src/ecs/systems.rs
git commit -m "feat: implement AABB collision_system and transform_system

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 10: Sprite Geometry Helpers & UV Tests

**Files:**
- Modify: `src/renderer/sprite.rs`

- [ ] **Step 10.1: Write failing tests for uv_rect and build_quad**

```rust
// src/renderer/sprite.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_rect_first_frame_is_top_left() {
        // 4 cols, 2 rows → frame 0 = col 0, row 0
        let uv = uv_rect(0, 4, 2);
        assert!((uv[0] - 0.0).abs() < 1e-6); // u0
        assert!((uv[1] - 0.0).abs() < 1e-6); // v0
        assert!((uv[2] - 0.25).abs() < 1e-6); // u1 = 1/4
        assert!((uv[3] - 0.5).abs() < 1e-6);  // v1 = 1/2
    }

    #[test]
    fn uv_rect_frame_five_in_4x2_sheet() {
        // frame 5: col = 5%4 = 1, row = 5/4 = 1
        // u0 = 1/4 = 0.25, v0 = 1/2 = 0.5
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
        // bottom-left vertex: normal has u0, flipped has u1
        assert!((normal[0].uv[0] - 0.0).abs() < 1e-6);
        assert!((flipped[0].uv[0] - 0.5).abs() < 1e-6);
    }
}
```

- [ ] **Step 10.2: Run tests to verify they fail**

```bash
cargo test renderer::sprite::tests
```

Expected: FAIL

- [ ] **Step 10.3: Implement sprite.rs**

```rust
// src/renderer/sprite.rs
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
```

- [ ] **Step 10.4: Run tests to verify they pass**

```bash
cargo test renderer::sprite::tests
```

Expected: 4 tests pass

- [ ] **Step 10.5: Commit**

```bash
git add src/renderer/sprite.rs
git commit -m "feat: add sprite UV and quad geometry helpers

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 11: Sprite WGSL Shader

**Files:**
- Modify: `src/shaders/sprite.wgsl`

> Shaders are verified at runtime (wgpu compiles them on device creation). No unit test possible.

- [ ] **Step 11.1: Write sprite.wgsl**

```wgsl
// src/shaders/sprite.wgsl

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@group(1) @binding(0)
var t_sprite: texture_2d<f32>;
@group(1) @binding(1)
var s_sprite: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_sprite, s_sprite, in.uv) * in.color;
}
```

- [ ] **Step 11.2: Verify compilation**

```bash
cargo check
```

Expected: No errors

- [ ] **Step 11.3: Commit**

```bash
git add src/shaders/sprite.wgsl
git commit -m "feat: add sprite WGSL vertex and fragment shader

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 12: GpuTexture & Renderer Init

**Files:**
- Modify: `src/renderer/texture.rs`
- Modify: `src/renderer/mod.rs`

> GPU code is verified at runtime via the sprite_demo example.

- [ ] **Step 12.1: Implement GpuTexture**

```rust
// src/renderer/texture.rs
use wgpu::*;

pub struct GpuTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub width: u32,
    pub height: u32,
}

impl GpuTexture {
    pub fn from_bytes(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, image::ImageError> {
        let img = image::load_from_memory(bytes)?.to_rgba8();
        let (width, height) = img.dimensions();
        let size = Extent3d { width, height, depth_or_array_layers: 1 };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            texture.as_image_copy(),
            &img,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest, // pixel-art style: no interpolation
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self { texture, view, sampler, width, height })
    }
}
```

- [ ] **Step 12.2: Implement Renderer**

```rust
// src/renderer/mod.rs
pub mod pipeline;
pub mod sprite;
pub mod texture;
#[cfg(feature = "tilemap")]
pub mod tilemap;

use std::sync::Arc;
use wgpu::*;
use winit::window::Window;

pub struct Renderer {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub surface_format: TextureFormat,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // SAFETY: surface must not outlive the window; Arc ensures window stays alive
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await
            .expect("failed to create wgpu device");

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self { device, queue, surface, config, surface_format }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}
```

- [ ] **Step 12.3: Verify compilation**

```bash
cargo check
```

Expected: No errors

- [ ] **Step 12.4: Commit**

```bash
git add src/renderer/
git commit -m "feat: add GpuTexture upload and wgpu Renderer initialization

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 13: Sprite Render Pipeline

**Files:**
- Modify: `src/renderer/pipeline.rs`

> GPU pipeline — verified at runtime via sprite_demo.

- [ ] **Step 13.1: Implement SpritePipeline**

```rust
// src/renderer/pipeline.rs
use std::collections::HashMap;
use bytemuck::cast_slice;
use glam::Mat4;
use wgpu::*;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use crate::renderer::sprite::{SpriteVertex, build_quad, uv_rect};
use crate::renderer::texture::GpuTexture;

// 6 indices per quad (2 triangles): 0,1,2 and 0,2,3
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
const MAX_SPRITES: usize = 10_000;

pub struct SpritePipeline {
    pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    texture_bind_group_layout: BindGroupLayout,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    texture_cache: HashMap<u64, (GpuTexture, BindGroup)>,
}

impl SpritePipeline {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(include_wgsl!("../shaders/sprite.wgsl"));

        // Camera uniform: orthographic projection matrix
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("camera"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera_bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("texture_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite_layout"),
            bind_group_layouts: &[&camera_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SpriteVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pre-allocate vertex buffer for MAX_SPRITES quads
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("sprite_vbo"),
            size: (MAX_SPRITES * 4 * std::mem::size_of::<SpriteVertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Pre-build index buffer for MAX_SPRITES quads
        let indices: Vec<u16> = (0..MAX_SPRITES as u16)
            .flat_map(|i| QUAD_INDICES.map(|idx| idx + i * 4))
            .collect();
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("sprite_ibo"),
            contents: cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout: texture_bgl,
            vertex_buffer,
            index_buffer,
            texture_cache: HashMap::new(),
        }
    }

    /// Upload a texture if not already cached. Call this before `draw()` so the
    /// draw pass only needs shared borrows of `self`.
    pub fn ensure_uploaded(
        &mut self,
        device: &Device,
        queue: &Queue,
        handle_id: u64,
        bytes: &[u8],
        label: &str,
    ) {
        if self.texture_cache.contains_key(&handle_id) { return; }
        let Ok(gpu_tex) = GpuTexture::from_bytes(device, queue, bytes, label) else { return };
        let bg = device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&gpu_tex.view) },
                BindGroupEntry { binding: 1, resource: BindingResource::Sampler(&gpu_tex.sampler) },
            ],
        });
        self.texture_cache.insert(handle_id, (gpu_tex, bg));
    }

    /// Update the orthographic camera to cover the window (origin = top-left).
    pub fn update_camera(&self, queue: &Queue, width: f32, height: f32) {
        let proj = Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0);
        queue.write_buffer(&self.camera_buffer, 0, cast_slice(&proj.to_cols_array_2d()));
    }

    /// Collect all (Sprite, Transform) entities into vertex batches and draw.
    pub fn draw<'a>(
        &'a mut self,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        world: &hecs::World,
        assets: &crate::assets::AssetServer,
        texture_bytes: &HashMap<u64, Vec<u8>>, // handle_id → raw PNG bytes
    ) {
        use crate::ecs::components::{Sprite, Transform};

        // Group entities by texture handle id
        let mut batches: HashMap<u64, Vec<[SpriteVertex; 4]>> = HashMap::new();
        for (_, (sprite, transform)) in world.query::<(&Sprite, &Transform)>().iter() {
            let sheet = match assets.get_sheet(&sprite.sheet) { Some(s) => s, None => continue };
            let handle_id = sprite.sheet.id();
            let uv = uv_rect(sprite.frame, sheet.columns, sheet.rows);
            let size = [sheet.frame_width as f32 * transform.scale.x,
                        sheet.frame_height as f32 * transform.scale.y];
            let quad = build_quad(
                [transform.position.x, transform.position.y],
                size, uv, sprite.color, sprite.flip_x, sprite.flip_y,
            );
            batches.entry(handle_id).or_default().push(quad);
        }

        // Phase 1: upload any missing textures (mutable; must complete before draw phase)
        for (handle_id, _) in &batches {
            if let Some(bytes) = texture_bytes.get(handle_id) {
                self.ensure_uploaded(device, queue, *handle_id, bytes, "sprite");
            }
        }

        // Phase 2: draw (only shared borrows of self from here — no borrow conflict)
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sprite_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        let mut vertex_offset: u64 = 0;
        for (handle_id, quads) in &batches {
            let Some((_, bg)) = self.texture_cache.get(handle_id) else { continue };

            let verts: Vec<SpriteVertex> = quads.iter().flat_map(|q| q.iter().copied()).collect();
            let byte_size = (verts.len() * std::mem::size_of::<SpriteVertex>()) as u64;
            queue.write_buffer(&self.vertex_buffer, vertex_offset, cast_slice(&verts));

            render_pass.set_bind_group(1, bg, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(vertex_offset..vertex_offset + byte_size));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..(quads.len() * 6) as u32, 0, 0..1);

            vertex_offset += byte_size;
        }
    }
}
```

- [ ] **Step 13.2: Verify compilation**

```bash
cargo check
```

Expected: No errors

- [ ] **Step 13.3: Commit**

```bash
git add src/renderer/pipeline.rs
git commit -m "feat: add SpritePipeline — bind groups, batched draw, orthographic camera

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 14: App Struct & System Registration

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 14.1: Write failing test**

```rust
// src/app.rs
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
```

- [ ] **Step 14.2: Run test to verify it fails**

```bash
cargo test app::tests
```

Expected: FAIL

- [ ] **Step 14.3: Implement App and AppContext**

```rust
// src/app.rs
use hecs::World;
use crate::{
    assets::AssetServer,
    ecs::resources::{Gravity, Time, CollisionEvents},
    input::InputState,
};

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
        Self { fixed_systems: Vec::new(), gravity: Gravity::default() }
    }

    pub fn with_gravity(mut self, gravity: Gravity) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn add_fixed_system<F: Fn(&mut AppContext) + 'static>(&mut self, f: F) -> &mut Self {
        self.fixed_systems.push(Box::new(f));
        self
    }

    pub fn fixed_system_count(&self) -> usize { self.fixed_systems.len() }

    /// Run one fixed-timestep tick: gravity → user systems (see prev-frame collisions) → collision → transform.
    pub fn tick(
        &self,
        world: &mut World,
        assets: &mut AssetServer,
        input: &InputState,
        collisions: &mut CollisionEvents,
        time: &mut Time,
        dt: f32,
    ) {
        use crate::ecs::systems::{gravity_system, collision_system, transform_system};

        time.fixed_delta = dt;
        time.elapsed += dt;

        gravity_system(world, &self.gravity, dt);

        // User systems see the *previous* frame's collision events.
        // Scope ensures the shared borrow of `collisions` ends before collision_system
        // takes a mutable borrow below.
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
        } // prev_collisions borrow released here

        collision_system(world, collisions); // generates this frame's events
        transform_system(world, dt);
    }
}

impl Default for App {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 14.4: Run test to verify it passes**

```bash
cargo test app::tests
```

Expected: 1 test passes

- [ ] **Step 14.5: Commit**

```bash
git add src/app.rs
git commit -m "feat: add App struct with fixed-timestep tick and system registration

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 15: Native Platform Layer & Run Loop

**Files:**
- Modify: `src/platform/mod.rs`
- Modify: `src/platform/native.rs`

> Integration-tested via the sprite_demo example.

- [ ] **Step 15.1: Write platform/mod.rs**

```rust
// src/platform/mod.rs
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
```

- [ ] **Step 15.2: Write platform/native.rs**

```rust
// src/platform/native.rs
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};
use hecs::World;
use crate::{
    app::{App, AppContext},
    assets::AssetServer,
    ecs::resources::{CollisionEvents, Time},
    ecs::systems::animation_system,
    input::InputState,
    renderer::{Renderer, pipeline::SpritePipeline},
};

const FIXED_DT: f32 = 1.0 / 60.0;

pub struct NativeApp {
    pub app: App,
    pub world: World,
    pub assets: AssetServer,
    pub texture_bytes: HashMap<u64, Vec<u8>>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    pipeline: Option<SpritePipeline>,
    input: InputState,
    collisions: CollisionEvents,
    time: Time,
    accumulator: f32,
    last_frame: Instant,
}

impl NativeApp {
    pub fn new(app: App, world: World, assets: AssetServer) -> Self {
        Self {
            app, world, assets,
            texture_bytes: HashMap::new(),
            window: None, renderer: None, pipeline: None,
            input: InputState::default(),
            collisions: CollisionEvents::default(),
            time: Time::default(),
            accumulator: 0.0,
            last_frame: Instant::now(),
        }
    }
}

impl ApplicationHandler for NativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(Window::default_attributes().with_title("Abura")).unwrap()
        );
        let renderer = pollster::block_on(Renderer::new(window.clone()));
        let pipeline = SpritePipeline::new(&renderer.device, renderer.surface_format);
        pipeline.update_camera(
            &renderer.queue,
            renderer.config.width as f32,
            renderer.config.height as f32,
        );
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.pipeline = Some(pipeline);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let (Some(r), Some(p)) = (&mut self.renderer, &mut self.pipeline) {
                    r.resize(size.width, size.height);
                    p.update_camera(&r.queue, size.width as f32, size.height as f32);
                }
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key: PhysicalKey::Code(code), state, .. }, ..
            } => {
                match state {
                    ElementState::Pressed => self.input.keyboard.press(code),
                    ElementState::Released => self.input.keyboard.release(code),
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let frame_dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
                self.last_frame = now;
                self.time.delta = frame_dt;
                self.accumulator += frame_dt;

                // Fixed-timestep update
                while self.accumulator >= FIXED_DT {
                    self.app.tick(
                        &mut self.world,
                        &mut self.assets,
                        &self.input,
                        &mut self.collisions,
                        &mut self.time,
                        FIXED_DT,
                    );
                    self.input.end_frame();
                    self.accumulator -= FIXED_DT;
                }

                // Render
                if let (Some(r), Some(p)) = (&self.renderer, &mut self.pipeline) {
                    animation_system(&mut self.world, frame_dt);

                    let output = match r.surface.get_current_texture() {
                        Ok(t) => t,
                        Err(_) => return,
                    };
                    let view = output.texture.create_view(&Default::default());
                    let mut encoder = r.device.create_command_encoder(&Default::default());

                    p.draw(&r.device, &r.queue, &view, &mut encoder,
                           &self.world, &self.assets, &self.texture_bytes);

                    r.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }

                if let Some(w) = &self.window { w.request_redraw(); }
            }

            _ => {}
        }
    }
}

pub fn run(native_app: NativeApp) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = native_app;
    event_loop.run_app(&mut app).unwrap();
}
```

- [ ] **Step 15.3: Add `pollster` dependency to Cargo.toml**

Add under `[dependencies]`:

```toml
pollster = "0.3"
```

- [ ] **Step 15.4: Verify compilation**

```bash
cargo check
```

Expected: No errors

- [ ] **Step 15.5: Commit**

```bash
git add src/platform/ Cargo.toml
git commit -m "feat: add native platform layer with winit event loop and fixed-timestep run loop

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 16: Tilemap Support (feature flag)

**Files:**
- Modify: `src/renderer/tilemap.rs`

- [ ] **Step 16.1: Write failing test for tile_uv**

```rust
// src/renderer/tilemap.rs
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
```

- [ ] **Step 16.2: Run tests to verify they fail**

```bash
cargo test --features tilemap renderer::tilemap::tests
```

Expected: FAIL

- [ ] **Step 16.3: Implement tilemap.rs**

```rust
// src/renderer/tilemap.rs
use glam::Vec2;
use crate::assets::Handle;
use crate::ecs::components::SpriteSheet;

pub struct TileMap {
    pub sheet: Handle<SpriteSheet>,
    pub tiles: Vec<u32>,   // flat grid, row-major
    pub width: u32,        // in tiles
    pub height: u32,
    pub tile_size: Vec2,
    pub dirty: bool,       // true when tiles changed — signals vertex buffer rebuild
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
```

- [ ] **Step 16.4: Run tests to verify they pass**

```bash
cargo test --features tilemap renderer::tilemap::tests
```

Expected: 2 tests pass

- [ ] **Step 16.5: Commit**

```bash
git add src/renderer/tilemap.rs
git commit -m "feat: add TileMap component and tile_uv helper (feature: tilemap)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 17: WASM Platform Layer

**Files:**
- Modify: `src/platform/wasm.rs`

- [ ] **Step 17.1: Implement wasm.rs**

```rust
// src/platform/wasm.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("Abura WASM initialized");
}

pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, JsValue> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().unwrap();
    let resp: Response = JsFuture::from(window.fetch_with_request(&request))
        .await?
        .dyn_into()?;
    let buffer = JsFuture::from(resp.array_buffer()?).await?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}
```

- [ ] **Step 17.2: Add wasm32 target if not installed**

```bash
rustup target add wasm32-unknown-unknown
```

- [ ] **Step 17.3: Verify WASM compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

Expected: No errors

- [ ] **Step 17.4: Commit**

```bash
git add src/platform/wasm.rs
git commit -m "feat: add WASM platform layer — panic hook, logging, async fetch

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 18: sprite_demo Example

**Files:**
- Modify: `examples/sprite_demo/main.rs`
- Create: `examples/sprite_demo/assets/` (placeholder sprite)

> This is the end-to-end integration test for the engine. Must compile and run.

- [ ] **Step 18.1: Create a placeholder spritesheet**

Create a 128×128 PNG with 4 rows × 4 cols (32×32 frames) at `examples/sprite_demo/assets/player.png`. Any valid PNG will do — use an image editor or this shell command if `convert` (ImageMagick) is available:

```bash
mkdir -p examples/sprite_demo/assets
convert -size 128x128 xc:red examples/sprite_demo/assets/player.png
```

If ImageMagick is not available, copy any 128×128 PNG to that path.

- [ ] **Step 18.2: Write the example**

```rust
// examples/sprite_demo/main.rs
use abura::{
    App, AppContext,
    AssetServer, Animator, AnimationClip, Collider, GravityScale,
    Sprite, SpriteSheet, Transform, Velocity,
};
use abura::ecs::resources::Gravity;
use abura::input::KeyCode;
use abura::platform::native::{NativeApp, run};
use glam::Vec2;
use hecs::World;
use std::collections::HashMap;

fn main() {
    env_logger::init();

    let mut assets = AssetServer::new();
    let sheet = assets.load_sheet(
        "assets/player.png",
        SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 4 },
    );

    // Animator with run and idle clips
    let mut animator = Animator::new();
    animator.add_clip("idle", AnimationClip { frames: vec![0], fps: 1.0, looping: true });
    animator.add_clip("run",  AnimationClip { frames: vec![1, 2, 3], fps: 8.0, looping: true });
    animator.play("idle");

    let mut world = World::new();
    world.spawn((
        Transform { position: Vec2::new(320.0, 240.0), scale: Vec2::ONE, rotation: 0.0 },
        Sprite { sheet: sheet.clone(), frame: 0, color: [1.0; 4], flip_x: false, flip_y: false },
        animator,
        Velocity::default(),
        GravityScale { scale: 1.0 },
        Collider { half_extents: Vec2::new(14.0, 14.0) },
    ));

    // Ground platform (static, no gravity)
    world.spawn((
        Transform { position: Vec2::new(320.0, 450.0), scale: Vec2::ONE, rotation: 0.0 },
        Collider { half_extents: Vec2::new(300.0, 16.0) },
    ));

    let mut app = App::new();

    // Player movement system
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

    // Load sprite bytes for renderer
    let mut texture_bytes: HashMap<u64, Vec<u8>> = HashMap::new();
    let sprite_bytes = std::fs::read("examples/sprite_demo/assets/player.png")
        .expect("player.png not found — see Step 18.1");
    texture_bytes.insert(sheet.id(), sprite_bytes);

    let mut native = NativeApp::new(app, world, assets);
    native.texture_bytes = texture_bytes;

    run(native);
}
```

- [ ] **Step 18.3: Build the example**

```bash
cargo build --example sprite_demo
```

Expected: builds successfully

- [ ] **Step 18.4: Run it**

```bash
cargo run --example sprite_demo
```

Expected: a window opens showing a red square (placeholder sprite), which falls due to gravity. Left/right arrows move it horizontally.

- [ ] **Step 18.5: Run the full test suite one final time**

```bash
cargo test
```

Expected: all unit tests pass

- [ ] **Step 18.6: Commit**

```bash
git add examples/
git commit -m "feat: add sprite_demo end-to-end example — animated player with gravity and movement

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Test Coverage Summary

| Task | Testable without GPU | How |
|------|---------------------|-----|
| 2 | Yes | `Transform::default`, `Tag` equality, `GravityScale` default |
| 3 | Yes | `SpriteSheet::frame_pixel_rect`, `Animator::play`, `current_frame` |
| 4 | Yes | `Gravity` default, `CollisionEvents::clear`, `Time` default |
| 5 | Yes | `just_pressed`, `just_released`, `mouse delta` |
| 6 | Yes | Asset handle deduplication and metadata retrieval |
| 7 | Yes | Animation frame advancement, looping, clamping |
| 8 | Yes | Gravity velocity accumulation, scale 0, missing component |
| 9 | Yes | AABB overlap, no-overlap, exact-edge, velocity application |
| 10 | Yes | UV rect normalization, quad vertex count, flip_x |
| 14 | Yes | Tile UV first/mid tile |
| 12, 13, 15 | GPU required | `cargo check` + sprite_demo runtime |
