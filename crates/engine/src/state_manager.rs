use std::sync::Mutex;

use crate::{
    components::Component,
    entity::EntityID,
    systems::{ClientSystems, SubsystemType},
};

pub struct Event {
    pub component: Component,
    pub entity_id: EntityID,
    pub system_type: SubsystemType,
}

pub struct EventSender {
    event_queue: Mutex<Vec<Event>>,
}

impl EventSender {
    pub fn new() -> Self {
        EventSender {
            event_queue: Mutex::new(Vec::new()),
        }
    }

    pub fn push(&self, event: Event) {
        self.event_queue.lock().unwrap().push(event);
    }
}

pub struct StateManager {
    pub sender: EventSender,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            sender: EventSender::new(),
        }
    }

    pub fn distribute(&self, systems: &mut ClientSystems) {
        let mut event_queue = self.sender.event_queue.lock().unwrap();

        for event in &*event_queue {
            systems.receive(event.entity_id, &event.component);
        }

        event_queue.clear();
    }
}

pub trait Listener {
    fn receive(&mut self, entity_id: EntityID, component: &Component);
}
