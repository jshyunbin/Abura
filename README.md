# 油 Abura

A minimal, fast-to-ship 2D game engine in Rust.

## Features

- **ECS architecture** via [`hecs`](https://github.com/Ralith/hecs) — plain Rust structs as components, no macros
- **Spritesheet animation** as a first-class feature — `Animator`, `AnimationClip`, per-entity playback
- **AABB collision detection** — pair events each fixed step, strict overlap (touching edges = no collision)
- **Gravity system** — per-entity `GravityScale` multiplier, global `Gravity` resource
- **Fixed-timestep game loop** — 60 Hz physics, render at display rate
- **Single codebase** targeting native (macOS, Windows, Linux) and WebAssembly
- **Sprite batching** — sorted by texture, one draw call per texture group
- **Tilemap support** (optional `tilemap` feature flag)

## Non-goals

- Shadows, lighting, or advanced GPU effects
- Full physics simulation (no joints, rigidbodies, or friction)
- Pixel-perfect collision
- Audio (designed as a future optional plugin)
- 3D rendering

## Architecture

```
┌─────────────────────────────────────┐
│         Game Code (user)            │
│  ECS systems, components, assets    │
└────────────────┬────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│           Engine Layer              │
│  App · game loop · system scheduler │
└────────┬───────────────┬────────────┘
         ↓               ↓
┌────────────────┐ ┌─────────────────┐
│   ECS Layer    │ │ Renderer Layer  │
│  hecs · World  │ │ wgpu · sprite   │
│  Query · World │ │ batcher · atlas │
└────────┬───────┘ └────────┬────────┘
         └────────┬──────────┘
                  ↓
┌─────────────────────────────────────┐
│          Platform Layer             │
│  winit · event loop · InputState    │
│  WASM canvas / native window        │
└─────────────────────────────────────┘
```

All platform differences are isolated in `src/platform/` — game code never contains `#[cfg(target_arch = "wasm32")]`.

## Quick Start

```rust
use abura::{App, AppContext, AssetServer, Sprite, SpriteSheet, Transform, Velocity};
use abura::platform::native::{NativeApp, run};
use glam::Vec2;
use hecs::World;
use std::collections::HashMap;

fn main() {
    let mut assets = AssetServer::new();
    let sheet = assets.load_sheet(
        "assets/player.png",
        SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 4 },
    );

    let mut world = World::new();
    world.spawn((
        Transform { position: Vec2::new(320.0, 240.0), scale: Vec2::ONE, rotation: 0.0 },
        Sprite { sheet: sheet.clone(), frame: 0, color: [1.0; 4], flip_x: false, flip_y: false },
    ));

    let app = App::new();

    let mut texture_bytes = HashMap::new();
    texture_bytes.insert(sheet.id(), std::fs::read("assets/player.png").unwrap());

    let mut native = NativeApp::new(app, world, assets);
    native.texture_bytes = texture_bytes;
    run(native);
}
```

## Built-in Components

| Component | Fields |
|-----------|--------|
| `Transform` | `position: Vec2`, `scale: Vec2`, `rotation: f32` |
| `Sprite` | `sheet: Handle<SpriteSheet>`, `frame: u32`, `color: [f32;4]`, `flip_x: bool`, `flip_y: bool` |
| `Animator` | clips, current clip, frame index, elapsed time |
| `Collider` | `half_extents: Vec2` (AABB relative to Transform position) |
| `Velocity` | `value: Vec2` — applied to position each fixed step |
| `GravityScale` | `scale: f32` — multiplier for the global `Gravity` resource |
| `Tag` | `Tag(pub u64)` — opaque numeric tag |

## Built-in Resources

| Resource | Description |
|----------|-------------|
| `InputState` | Keyboard, mouse, gamepad state — updated each frame |
| `CollisionEvents` | `pairs: Vec<(Entity, Entity)>` — overlapping AABB pairs |
| `AssetServer` | Load and cache spritesheets |
| `Time` | `delta`, `fixed_delta`, `elapsed` |
| `Gravity` | Global gravity acceleration (default `(0.0, -980.0)`) |

## Running the Example

```bash
# Native
cargo run --example sprite_demo

# With tilemap feature
cargo run --example sprite_demo --features tilemap
```

The demo opens a window with a red placeholder sprite. It falls due to gravity; use arrow keys to move left and right.

## WebAssembly Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir www/ \
  target/wasm32-unknown-unknown/release/abura.wasm
```

Then serve `www/` with any static file server and open the HTML page.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `winit` | Cross-platform windowing + event loop |
| `wgpu` | GPU rendering — WebGPU on WASM, Vulkan/Metal/DX12 on native |
| `hecs` | Lightweight ECS |
| `glam` | Math — `Vec2`, `Mat4` |
| `image` | Texture loading from PNG/JPEG |
| `bytemuck` | Safe transmutes for GPU vertex data |
| `gilrs` | Gamepad input (native only) |
| `wasm-bindgen` | WASM bridge (WASM target only) |

## Feature Flags

| Flag | Description |
|------|-------------|
| `tilemap` | Enables `TileMap` component and `tile_uv` helper |
| `audio` | Reserved for a future audio plugin (not yet implemented) |

## License

MIT
