mod action;
mod binding;
mod event;
mod map;

use std::path::Path;

use gg_math::Vec2;
use gg_util::ahash::AHashSet;
use gg_util::eyre::{Context, Result};
use winit::event::{KeyboardInput, ModifiersState, MouseScrollDelta, WindowEvent};

use self::action::ActionRegistry;
pub use self::action::{Action, ActionKind};
use self::binding::BindingElement;
pub use self::event::*;
use self::map::InputMap;

#[derive(Debug, Default)]
pub struct Input {
    actions: ActionRegistry,
    map: InputMap,
    state: State,
    events: Vec<Event>,
}

#[derive(Debug, Default)]
struct State {
    actions: AHashSet<Action>,
    new_actions: AHashSet<Action>,
    elements: AHashSet<BindingElement>,
    modifiers: ModifiersState,
    mouse_pos: Vec2<f32>,
}

impl Input {
    pub fn new() -> Input {
        Input::default()
    }

    pub fn register_action<A: ActionKind>(&mut self) {
        self.actions.register::<A>();
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let data = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("cannot read {}", path.display()))?;
        self.map.parse(&self.actions, &data)
    }

    pub fn begin_frame(&mut self) {
        self.events.clear();
    }

    pub fn process_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.state.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
            }

            WindowEvent::ModifiersChanged(v) => {
                self.state.modifiers = v;
                self.update_actions();
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.process_scroll(delta);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.process_mouse_input(state, button);
            }

            WindowEvent::KeyboardInput { input, .. } => {
                self.process_keyboard_input(input);
            }

            _ => {}
        }
    }

    fn process_scroll(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
            MouseScrollDelta::PixelDelta(v) => Vec2::new(v.x as f32, v.y as f32),
        };

        self.events.push(Event::Scroll(ScrollEvent { delta }));
    }

    fn process_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        self.events.push(Event::Mouse(MouseEvent { state, button }));
        self.process_element(state, BindingElement::Mouse(button));
    }

    fn process_keyboard_input(&mut self, input: KeyboardInput) {
        let code = match input.virtual_keycode {
            Some(v) => v,
            None => return,
        };

        self.events.push(Event::Keyboard(KeyboardEvent {
            state: input.state,
            code,
        }));

        self.process_element(input.state, BindingElement::Keyboard(code));
    }

    fn process_element(&mut self, state: ElementState, element: BindingElement) {
        match state {
            ElementState::Pressed => {
                self.state.elements.insert(element);
            }
            ElementState::Released => {
                self.state.elements.remove(&element);
            }
        }

        self.update_actions();
    }

    fn update_actions(&mut self) {
        let old_set = &mut self.state.actions;
        let new_set = &mut self.state.new_actions;

        new_set.clear();

        for action in self.map.filter(&self.state.elements, self.state.modifiers) {
            new_set.insert(action);
        }

        for &action in old_set.difference(&new_set) {
            self.events.push(Event::Action(ActionEvent {
                action,
                state: ElementState::Released,
            }));
        }

        for &action in new_set.difference(&old_set) {
            self.events.push(Event::Action(ActionEvent {
                action,
                state: ElementState::Pressed,
            }));
        }

        std::mem::swap(old_set, new_set);
    }

    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        self.events.iter().copied()
    }

    pub fn is_action_pressed(&self, action: impl Into<Action>) -> bool {
        self.state.actions.contains(&action.into())
    }

    pub fn has_action_pressed(&self, action: impl Into<Action>) -> bool {
        let action = action.into();
        self.events().any(|ev| ev.pressed_action(action))
    }

    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.state.elements.contains(&BindingElement::Keyboard(key))
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.state.elements.contains(&BindingElement::Mouse(button))
    }

    pub fn mouse_pos(&self) -> Vec2<f32> {
        self.state.mouse_pos
    }
}
