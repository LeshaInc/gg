use gg_math::Vec2;
pub use winit::event::{ElementState, MouseButton, VirtualKeyCode};

use crate::Action;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    Scroll(ScrollEvent),
    Char(char),
    Action(ActionEvent),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardEvent {
    pub state: ElementState,
    pub code: VirtualKeyCode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseEvent {
    pub state: ElementState,
    pub button: MouseButton,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollEvent {
    pub delta: Vec2<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionEvent {
    pub state: ElementState,
    pub action: Action,
}
