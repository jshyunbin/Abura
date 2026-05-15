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
    app::App,
    assets::AssetServer,
    ecs::resources::{CollisionEvents, Time},
    ecs::systems::animation_system,
    input::InputState,
    renderer::{Renderer, pipeline::SpritePipeline},
};

#[allow(dead_code)]
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
            app,
            world,
            assets,
            texture_bytes: HashMap::new(),
            window: None,
            renderer: None,
            pipeline: None,
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
            event_loop
                .create_window(Window::default_attributes().with_title("Abura"))
                .unwrap(),
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

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let (Some(r), Some(p)) = (&mut self.renderer, &mut self.pipeline) {
                    r.resize(size.width, size.height);
                    p.update_camera(&r.queue, size.width as f32, size.height as f32);
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => self.input.keyboard.press(code),
                ElementState::Released => self.input.keyboard.release(code),
            },

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let frame_dt = now
                    .duration_since(self.last_frame)
                    .as_secs_f32()
                    .min(0.1);
                self.last_frame = now;
                self.time.delta = frame_dt;
                self.accumulator += frame_dt;

                while self.accumulator >= FIXED_DT {
                    self.app.tick(
                        &mut self.world,
                        &mut self.assets,
                        &self.input,
                        &mut self.collisions,
                        &mut self.time,
                        FIXED_DT,
                    );
                    self.accumulator -= FIXED_DT;
                }
                self.input.end_frame();

                if let (Some(r), Some(p)) = (&self.renderer, &mut self.pipeline) {
                    animation_system(&mut self.world, frame_dt);

                    let output = match r.surface.get_current_texture() {
                        Ok(t) => t,
                        Err(_) => return,
                    };
                    let view = output.texture.create_view(&Default::default());
                    let mut encoder =
                        r.device.create_command_encoder(&Default::default());

                    p.draw(
                        &r.device,
                        &r.queue,
                        &view,
                        &mut encoder,
                        &self.world,
                        &self.assets,
                        &self.texture_bytes,
                    );

                    r.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
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
