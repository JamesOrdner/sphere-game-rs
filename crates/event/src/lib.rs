use std::thread::{self, ThreadId};

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

static mut EVENT_SENDERS: Vec<(ThreadId, EventSender)> = Vec::new();

pub fn push_event(entity_id: EntityId, component: Component) {
    let thread_id = thread::current().id();

    unsafe {
        for event_sender in &mut EVENT_SENDERS {
            if event_sender.0 == thread_id {
                event_sender.1.push(entity_id, component);
                return;
            }
        }
    }

    unreachable!("push_event() called from invalid thread")
}

pub struct EventManager;

impl EventManager {
    pub fn new(thread_ids: &[ThreadId]) -> Self {
        unsafe {
            EVENT_SENDERS.clear();
            for thread_id in thread_ids {
                EVENT_SENDERS.push((*thread_id, EventSender::new()));
            }
        }

        Self {}
    }

    pub fn distribute<F>(&mut self, mut event_handler: F)
    where
        F: FnMut(EntityId, &Component),
    {
        unsafe {
            for event_sender in &mut EVENT_SENDERS {
                for event in &mut event_sender.1.event_queue {
                    event_handler(event.entity_id, &event.component);
                }
                event_sender.1.event_queue.clear();
            }
        }
    }
}

pub trait EventListener {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component);
}
