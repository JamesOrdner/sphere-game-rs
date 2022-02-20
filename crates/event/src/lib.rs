use std::{cell::Cell, ptr};

use component::Component;
use entity::EntityId;

struct Event {
    pub component: Component,
    pub entity_id: EntityId,
}

struct EventSender {
    event_queue: Vec<Event>,
}

impl EventSender {
    pub fn new() -> Self {
        EventSender {
            event_queue: Vec::new(),
        }
    }

    pub fn push(&mut self, entity_id: EntityId, component: Component) {
        self.event_queue.push(Event {
            entity_id,
            component,
        });
    }
}

thread_local! {
    static EVENT_SENDER: Cell<*mut EventSender> = Cell::new(ptr::null_mut())
}

static mut EVENT_SENDERS: Vec<EventSender> = Vec::new();

/// SAFETY: must be externally synchronized
pub unsafe fn add_event_sender() {
    EVENT_SENDERS.push(EventSender::new());
    EVENT_SENDER.with(|sender| sender.set(EVENT_SENDERS.last_mut().unwrap()));
}

pub fn push_event(entity_id: EntityId, component: Component) {
    EVENT_SENDER.with(|sender| unsafe {
        sender
            .get()
            .as_mut()
            .unwrap_unchecked()
            .push(entity_id, component)
    });
}

pub struct EventManager;

impl EventManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn distribute<F>(&mut self, mut event_handler: F)
    where
        F: FnMut(EntityId, &Component),
    {
        unsafe {
            for event_sender in &mut EVENT_SENDERS {
                for event in &mut event_sender.event_queue {
                    event_handler(event.entity_id, &event.component);
                }
                event_sender.event_queue.clear();
            }
        }
    }
}

pub trait EventListener {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component);
}
