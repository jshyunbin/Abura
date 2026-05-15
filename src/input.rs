use glam::Vec2;
use std::collections::HashSet;

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

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

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }

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
        if self.pressed.insert(button) {
            self.just_pressed.insert(button);
        }
    }

    pub fn release(&mut self, button: MouseButton) {
        self.pressed.remove(&button);
        self.just_released.insert(button);
    }

    pub fn is_pressed(&self, b: MouseButton) -> bool {
        self.pressed.contains(&b)
    }

    pub fn just_pressed(&self, b: MouseButton) -> bool {
        self.just_pressed.contains(&b)
    }

    pub fn just_released(&self, button: MouseButton) -> bool {
        self.just_released.contains(&button)
    }

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
    pub fn button_pressed(&self, id: u32) -> bool {
        self.buttons.contains(&id)
    }

    pub fn axis_value(&self, id: usize) -> f32 {
        self.axes.get(id).copied().unwrap_or(0.0)
    }
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
        assert!(kb.is_pressed(KeyCode::Space)); // still held
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
    fn mouse_just_pressed_fires_once() {
        let mut mouse = MouseState::default();
        mouse.press(MouseButton::Left);
        assert!(mouse.just_pressed(MouseButton::Left));
        // Simulate end of frame: clears just_pressed, button stays in pressed
        mouse.end_frame();
        // The OS may re-send a press event while the button is still held;
        // just_pressed must not fire again.
        mouse.press(MouseButton::Left);
        assert!(
            !mouse.just_pressed(MouseButton::Left),
            "just_pressed must not fire again when already held"
        );
    }

    #[test]
    fn mouse_delta_clears_each_frame() {
        let mut m = MouseState::default();
        m.delta = glam::Vec2::new(5.0, 3.0);
        m.end_frame();
        assert_eq!(m.delta, glam::Vec2::ZERO);
    }
}
