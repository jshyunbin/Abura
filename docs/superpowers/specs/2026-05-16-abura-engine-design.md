# Abura — Minimal 2D Game Engine Design

**Date:** 2026-05-16
**Target:** Arcade / action games
**Platforms:** Native (macOS, Windows, Linux) + WebAssembly

---

## 1. Goals & Non-Goals

### Goals
- Minimal, fast-to-ship 2D game engine in Rust
- Spritesheet animation as a first-class feature
- ECS architecture for performance with many entities
- Single codebase targeting both native and WASM
- AABB collision detection built in
- Basic tilemap support (optional feature flag)

### Non-Goals
- Shadows, lighting, or any advanced GPU effects
- Physics simulation (no gravity, joints, rigidbodies)
- Pixel-perfect collision (game code can layer this if needed)
- Audio (designed as a future optional plugin, not implemented now)
- 3D rendering of any kind

---

## 2. Architecture Overview

One crate (`abura`), four clear internal layers. All platform differences are isolated in the Platform layer — game code never contains `#[cfg(target_arch = "wasm32")]`.

```
┌─────────────────────────────────────┐
│         Game Code (user)            │
│  ECS systems, components, assets    │
└────────────────┬────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│           Engine Layer              │
│  App · game loop · system scheduler │
│  delta time · asset manager         │
└────────┬───────────────┬────────────┘
         ↓               ↓
┌────────────────┐ ┌─────────────────┐
│   ECS Layer    │ │ Renderer Layer  │
│  hecs · World  │ │ wgpu · sprite   │
│  Query · World │ │ batcher · atlas │
│                │ │ tilemap         │
└────────┬───────┘ └────────┬────────┘
         └────────┬──────────┘
                  ↓
┌─────────────────────────────────────┐
│          Platform Layer             │
│  winit · event loop · InputState    │
│  WASM canvas / native window        │
└──────────────┬──────────────────────┘
               ↓
     ┌─────────────────────┐
     │ Native              │ WASM
     │ Vulkan/Metal/DX12   │ WebGPU / WebGL2
     └─────────────────────┘
```

**Optional modules (Cargo feature flags):**
- `tilemap` — tile grid rendering
- `audio` — future plugin (not implemented)

---

## 3. Key Dependencies

| Crate | Purpose |
|-------|---------|
| `winit` | Cross-platform windowing + event loop (WASM + native) |
| `wgpu` | GPU rendering — WebGPU on WASM, Vulkan/Metal/DX12 on native |
| `hecs` | Lightweight ECS — no proc macros, fast queries |
| `glam` | Math — Vec2, Vec3, Mat4 |
| `image` | Texture loading from PNG/JPEG |
| `bytemuck` | Safe transmutes for GPU vertex data |
| `gilrs` | Gamepad input (native only) |
| `wasm-bindgen` | WASM bridge (WASM target only) |
| `web-sys` | Browser fetch API for async asset loading (WASM only) |

---

## 4. ECS Layer

Uses `hecs` as the ECS backend. The `World` is the central store for all entities and components. Systems are plain Rust functions that receive `&World` or `&mut World` plus resources.

### Built-in Components

| Component | Fields |
|-----------|--------|
| `Transform` | `position: Vec2`, `scale: Vec2`, `rotation: f32` |
| `Sprite` | `sheet: Handle<SpriteSheet>`, `frame: u32`, `color: [f32;4]`, `flip_x: bool`, `flip_y: bool` |
| `Animator` | `clips: HashMap<String, AnimationClip>`, `current: String`, `frame_index: usize`, `elapsed: f32` |
| `Collider` | `half_extents: Vec2` (AABB relative to Transform position) |
| `Velocity` | `value: Vec2` — applied to `Transform.position` each fixed step (`pos += vel * dt`) |
| `Tag` | `Tag(pub u64)` — opaque numeric tag; game code casts its own enum discriminants in |

Game code adds its own components freely — no engine registration required.

### Built-in Resources

| Resource | Description |
|----------|-------------|
| `InputState` | Keyboard, mouse, gamepad state — written by Platform layer each frame |
| `CollisionEvents` | `pairs: Vec<(Entity, Entity)>` — overlapping AABB pairs, cleared each fixed step |
| `AssetServer` | Load and cache textures and spritesheets |
| `Time` | `delta: f32`, `fixed_delta: f32`, `elapsed: f32` |

---

## 5. Renderer Layer

### Sprite Rendering Pipeline (per frame)

1. Query all entities with `Sprite + Transform`
2. Sort by `Handle<SpriteSheet>` (minimise GPU texture bind changes)
3. Build a vertex buffer — each sprite is 4 vertices (quad) with UV coords computed from `frame` index into the spritesheet grid
4. Issue one batched draw call per texture group
5. No shadows, no lighting passes — just textured quads

### SpriteSheet

```rust
pub struct SpriteSheet {
    pub texture: Handle<Texture>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}
```

Frame `n` maps to UV rect: `(col * fw, row * fh)` where `col = n % columns`, `row = n / columns`.

### AnimationClip

```rust
pub struct AnimationClip {
    pub frames: Vec<u32>,   // frame indices into the spritesheet
    pub fps: f32,
    pub looping: bool,
}
```

### Animation System

Runs in the render phase (before sprite batch build):

```
for each entity with (mut Animator, mut Sprite):
    clip = animator.current_clip()
    animator.elapsed += delta_time
    if elapsed >= 1.0 / clip.fps:
        elapsed = 0.0
        advance frame_index
        if at end && looping  → wrap to 0
        if at end && !looping → clamp to last frame
    sprite.frame = clip.frames[frame_index]
```

Game code API: `animator.play("run")`, `animator.play("die")`.

### Tilemap (`tilemap` feature)

```rust
pub struct TileMap {
    pub sheet: Handle<SpriteSheet>,
    pub tiles: Vec<u32>,      // flat grid, row-major
    pub width: u32,           // in tiles
    pub height: u32,
    pub tile_size: Vec2,
}
```

- Vertex buffer built once, rebuilt only when `tiles` changes
- Drawn before sprites (background layer)
- Off-screen tiles frustum-culled before upload

---

## 6. Game Loop

Fixed-timestep accumulator pattern:

```
fixed_dt = 1.0 / 60.0
accumulator = 0.0

each frame:
    accumulator += frame_delta_time
    while accumulator >= fixed_dt:
        run_fixed_update_systems(fixed_dt)
        accumulator -= fixed_dt
    alpha = accumulator / fixed_dt
    run_render_systems(alpha)
```

**Fixed update system order:**
1. Input system (flush winit events → InputState)
2. User game logic systems
3. AABB collision system (write overlapping pairs to `CollisionEvents` resource)
4. Transform update (apply `Velocity`: `transform.position += velocity.value * fixed_dt`)

**Render system order:**
1. Animation system (advance Animator → write Sprite.frame)
2. Sprite batch builder
3. Tilemap renderer (if feature enabled)
4. wgpu command submit

---

## 7. Input System

`InputState` is a resource written by the Platform layer each frame and read by any ECS system.

```rust
pub struct InputState {
    pub keyboard: KeyboardState,  // is_pressed, just_pressed, just_released
    pub mouse: MouseState,        // position, delta, buttons
    pub gamepad: GamepadState,    // buttons, axes (gilrs, native only)
}
```

Identical API on WASM and native. No platform conditionals leak into game code.

---

## 8. Asset Manager

```rust
// Game code
let sheet: Handle<SpriteSheet> = assets.load("player.png", SpriteSheetDesc {
    frame_width: 32, frame_height: 32, columns: 8, rows: 4
});
```

- `Handle<T>` — cheap clone, typed opaque ID
- `AssetServer` deduplicates by path (same path → same handle)
- Native: synchronous filesystem load, GPU upload on first use
- WASM: async HTTP fetch via browser Fetch API; asset available the frame after it resolves. Engine skips rendering entities with unresolved handles.

---

## 9. WASM Deployment

**Build:**
```bash
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir www/ target/wasm32-unknown-unknown/release/abura.wasm
```

**HTML integration:**
```html
<canvas id="abura-canvas"></canvas>
<script type="module">
  import init from './www/abura.js';
  await init();
</script>
```

winit attaches to the canvas automatically via `web-sys`. wgpu targets WebGPU with WebGL2 fallback for broader browser support.

---

## 10. Plugin Architecture (Future: Audio)

The `audio` feature flag will introduce an `AudioPlugin` trait:

```rust
pub trait Plugin {
    fn build(&self, app: &mut App);
}
```

Game code registers plugins at startup:
```rust
App::new()
    .add_plugin(AudioPlugin::new())
    .run();
```

The engine itself does not depend on any plugin — they register their own systems and resources into the `App` at build time.

---

## 11. Crate Structure

```
abura/
├── Cargo.toml
├── src/
│   ├── lib.rs              # public re-exports
│   ├── app.rs              # App struct, game loop
│   ├── platform/
│   │   ├── mod.rs
│   │   ├── native.rs
│   │   └── wasm.rs
│   ├── renderer/
│   │   ├── mod.rs
│   │   ├── sprite.rs       # sprite batch
│   │   ├── texture.rs      # texture upload
│   │   └── tilemap.rs      # (feature: tilemap)
│   ├── ecs/
│   │   ├── mod.rs
│   │   ├── components.rs   # built-in components
│   │   └── systems.rs      # animation, collision, input
│   ├── input.rs            # InputState
│   └── assets.rs           # AssetServer, Handle<T>
└── examples/
    └── sprite_demo/        # minimal working example
```

---

## 12. Success Criteria

- [ ] Sprite from a spritesheet renders on screen (native)
- [ ] Same build runs in browser via WASM
- [ ] Spritesheet animation advances frames at correct FPS
- [ ] AABB collision events fire between overlapping entities
- [ ] 500+ animated sprites at 60fps on a mid-range machine
- [ ] Tilemap renders as background layer (when feature enabled)
- [ ] Input (keyboard + mouse) works identically on native and WASM
