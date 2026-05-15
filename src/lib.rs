pub mod app;
pub mod assets;
pub mod ecs;
pub mod input;
pub mod platform;
pub mod renderer;

pub use app::{App, AppContext};
pub use assets::{AssetServer, Handle};
pub use ecs::components::*;
pub use ecs::resources::*;
pub use ecs::systems::*;
pub use input::InputState;

#[cfg(not(target_arch = "wasm32"))]
pub use platform::native::{run, NativeApp};
