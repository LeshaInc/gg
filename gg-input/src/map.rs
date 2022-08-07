use gg_util::ahash::{AHashMap, AHashSet};
use gg_util::eyre::Result;
use winit::event::ModifiersState;

use crate::action::ActionRegistry;
use crate::binding::{Binding, BindingElement};
use crate::Action;

#[derive(Clone, Debug, Default)]
pub struct InputMap {
    map: AHashMap<BindingElement, Vec<(Binding, Action)>>,
}

impl InputMap {
    pub fn new() -> InputMap {
        InputMap::default()
    }

    pub fn parse(&mut self, actions: &ActionRegistry, data: &str) -> Result<()> {
        let list: Vec<(String, Binding)> = serde_json::from_str(data)?;

        for (action_name, binding) in list {
            if let Some(action) = actions.get(&action_name) {
                self.add_binding(binding, action);
            } else {
                tracing::warn!("no such action: {}", action_name);
            }
        }

        Ok(())
    }

    pub fn add_binding(&mut self, binding: Binding, action: Action) {
        self.remove_binding(binding, action);

        for element in binding.elements() {
            let bindings = self.map.entry(element).or_default();
            bindings.push((binding, action));
        }
    }

    pub fn remove_binding(&mut self, binding: Binding, action: Action) {
        for element in binding.elements() {
            let bindings = self.map.entry(element).or_default();
            bindings.retain(|&v| v != (binding, action))
        }
    }

    pub fn filter<'s: 'c, 'c>(
        &'s self,
        elements: &'c AHashSet<BindingElement>,
        modifiers: ModifiersState,
    ) -> impl Iterator<Item = Action> + 'c {
        elements
            .iter()
            .flat_map(|el| self.map.get(el))
            .flatten()
            .filter(move |(binding, _)| {
                modifiers.contains(binding.modifiers())
                    && binding.elements().all(|c| elements.contains(&c))
            })
            .map(|(_, action)| *action)
    }
}
