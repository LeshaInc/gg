use std::fmt::{self, Debug, Display};
use std::str::FromStr;

use gg_util::eyre::{bail, eyre, Report};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use winit::event::{ModifiersState, MouseButton, VirtualKeyCode};

const MAX_ELEMENTS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, DeserializeFromStr, SerializeDisplay)]
pub struct Binding {
    elements: [BindingElement; MAX_ELEMENTS],
    elements_len: usize,
    modifiers: ModifiersState,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BindingElement {
    Keyboard(VirtualKeyCode),
    Mouse(MouseButton),
}

impl Binding {
    pub fn elements(&self) -> impl Iterator<Item = BindingElement> {
        self.elements.into_iter().take(self.elements_len as usize)
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }
}

impl FromStr for Binding {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut modifiers = ModifiersState::empty();

        let mut elements = [BindingElement::Keyboard(VirtualKeyCode::Key1); MAX_ELEMENTS];
        let mut i = 0;

        for part in s.split('-') {
            if let Some(m) = parse_modifier(part) {
                modifiers |= m;
            } else {
                let element = parse_key(part)
                    .map(BindingElement::Keyboard)
                    .or_else(|| parse_mouse_button(part).map(BindingElement::Mouse))
                    .ok_or_else(|| eyre!("invalid binding element: {}", part))?;

                if i < elements.len() {
                    elements[i] = element;
                    i += 1;
                } else {
                    bail!("too many binding elements");
                }
            }
        }

        if i == 0 {
            bail!("empty binding");
        }

        Ok(Binding {
            elements,
            elements_len: i,
            modifiers,
        })
    }
}

impl Display for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_modifiers(self.modifiers, f)?;

        for (i, element) in self.elements().enumerate() {
            if !self.modifiers.is_empty() || i > 0 {
                f.write_str("-")?;
            }

            match element {
                BindingElement::Keyboard(key) => key.fmt(f)?,
                BindingElement::Mouse(btn) => display_mouse_button(btn, f)?,
            }
        }

        Ok(())
    }
}

fn parse_key(s: &str) -> Option<VirtualKeyCode> {
    serde_json::from_value(serde_json::json!(s)).ok()
}

fn parse_mouse_button(s: &str) -> Option<MouseButton> {
    Some(match s {
        "MouseLeft" => MouseButton::Left,
        "MouseMiddle" => MouseButton::Middle,
        "MouseRight" => MouseButton::Right,
        _ if s.starts_with("Mouse") => {
            let val = u16::from_str(&s[5..]).ok()?;
            MouseButton::Other(val)
        }
        _ => return None,
    })
}

fn display_mouse_button(v: MouseButton, f: &mut fmt::Formatter) -> fmt::Result {
    match v {
        MouseButton::Left => f.write_str("MouseLeft"),
        MouseButton::Middle => f.write_str("MouseMiddle"),
        MouseButton::Right => f.write_str("MouseRight"),
        MouseButton::Other(idx) => {
            write!(f, "Mouse{}", idx)
        }
    }
}

const MODIFIERS: [(&str, ModifiersState); 4] = [
    ("Ctrl", ModifiersState::CTRL),
    ("Alt", ModifiersState::ALT),
    ("Shift", ModifiersState::SHIFT),
    ("Logo", ModifiersState::LOGO),
];

fn parse_modifier(s: &str) -> Option<ModifiersState> {
    for &(pat, modifier) in &MODIFIERS {
        if s == pat {
            return Some(modifier);
        }
    }

    None
}

fn display_modifiers(mods: ModifiersState, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut tail = false;

    for &(pat, modifier) in &MODIFIERS {
        if mods.contains(modifier) {
            if tail {
                f.write_str("-")?;
            }

            f.write_str(pat)?;
            tail = true;
        }
    }

    Ok(())
}
