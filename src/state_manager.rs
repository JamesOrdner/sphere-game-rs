use crate::{
    components::{Component, ComponentRef, ComponentType},
    entity::EntityID,
    systems::{SystemType, Systems},
};

pub struct Event {
    pub entity_id: EntityID,
    pub component_type: ComponentType,
    pub system: SystemType,
}

pub struct Sender {
    event_queue: Vec<Event>,
}

impl Sender {
    pub fn new() -> Self {
        Sender {
            event_queue: Vec::new(),
        }
    }

    pub fn push(&mut self, event: Event) {
        self.event_queue.push(event);
    }
}

pub struct StateManager;

impl StateManager {
    pub fn new() -> Self {
        StateManager {}
    }

    pub fn distribute(&self, senders: &mut [Sender], systems: &mut Systems) {
        for sender in senders {
            for event in &sender.event_queue {
                if let Some(component_val) = get_component_val(event, systems) {
                    systems.receive_for_each_listener(event.entity_id, &component_val);
                }
            }

            sender.event_queue.clear();
        }
    }
}

fn get_component_val(event: &Event, systems: &Systems) -> Option<Component> {
    if let Some(component_ref) = get_component_ref(event, systems) {
        Some(Component::from_ref(component_ref))
    } else {
        None
    }
}

fn get_component_ref<'a>(event: &Event, systems: &'a Systems) -> Option<ComponentRef<'a>> {
    match event.system {
        SystemType::Input => systems
            .client
            .as_ref()
            .unwrap()
            .input
            .get(event.component_type, event.entity_id),
        SystemType::Physics => systems
            .core
            .physics
            .get(event.component_type, event.entity_id),
        _ => None,
    }
}

pub trait ComponentQuery {
    fn get(&self, component_type: ComponentType, entity_id: EntityID) -> Option<ComponentRef>;
}

pub trait Listener {
    fn receive(&mut self, entity_id: EntityID, component: &Component);
}
