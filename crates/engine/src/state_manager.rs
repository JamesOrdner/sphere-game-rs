use std::thread::ThreadId;

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
    event_queue: Vec<Event>,
}

impl EventSender {
    pub fn new() -> Self {
        EventSender {
            event_queue: Vec::new(),
        }
    }

    pub fn push(&mut self, entity_id: EntityID, component: Component, system_type: SubsystemType) {
        self.event_queue.push(Event {
            entity_id,
            component,
            system_type,
        });
    }
}

static mut EVENT_SENDERS: Vec<(ThreadId, EventSender)> = Vec::new();

pub fn push_event(
    entity_id: EntityID,
    component: Component,
    system_type: SubsystemType,
    thread_id: ThreadId,
) {
    unsafe {
        for event_sender in &mut EVENT_SENDERS {
            if event_sender.0 == thread_id {
                event_sender.1.push(entity_id, component, system_type);
                return;
            }
        }
    }

    unreachable!("push_event() called from invalid thread")
}

pub struct StateManager;

impl StateManager {
    pub fn new(thread_ids: Vec<ThreadId>) -> Self {
        unsafe {
            EVENT_SENDERS.clear();
            for thread_id in thread_ids {
                EVENT_SENDERS.push((thread_id, EventSender::new()));
            }
        }

        Self {}
    }

    pub fn sender(&mut self) -> &mut EventSender {
        unsafe { &mut EVENT_SENDERS[0].1 }
    }

    pub fn distribute(&mut self, systems: &mut ClientSystems) {
        unsafe {
            for event_sender in &mut EVENT_SENDERS {
                for event in &mut event_sender.1.event_queue {
                    systems.receive(event.entity_id, &event.component);
                }
                event_sender.1.event_queue.clear();
            }
        }
    }
}

pub trait Listener {
    fn receive(&mut self, entity_id: EntityID, component: &Component);
}
