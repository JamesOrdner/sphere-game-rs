use crate::{
    components::Component,
    entity::EntityID,
    systems::{SubsystemType, Systems},
};

pub struct Event {
    pub component: Component,
    pub entity_id: EntityID,
    pub system_type: SubsystemType,
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
        Self {}
    }

    pub fn distribute(&self, senders: &mut [Sender], systems: &mut Systems) {
        for sender in senders {
            for event in &sender.event_queue {
                systems.receive(event.entity_id, &event.component);
            }

            sender.event_queue.clear();
        }
    }
}

pub trait Listener {
    fn receive(&mut self, entity_id: EntityID, component: &Component);
}
