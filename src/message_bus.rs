use crate::common::EntityID;

pub enum Message {
    Location { entity_id: EntityID, x: f32, y: f32 },
    InputAcceleration { x: f32, y: f32 },
}

pub trait Receiver {
    fn receive(&mut self, messages: &[Message]);
}

pub struct MessageBus {}

impl MessageBus {
    pub fn new() -> Self {
        MessageBus {}
    }

    pub fn distribute<R>(&self, senders: &[Sender], receiver: &mut R)
    where
        R: Receiver,
    {
        // MVP until we hammer out an efficient message storage/distribution system
        for sender in senders {
            receiver.receive(&sender.message_queue);
        }
    }

    pub fn clear_queue(&self, senders: &mut [Sender]) {
        for sender in senders {
            sender.message_queue.clear();
        }
    }
}

pub struct Sender {
    message_queue: Vec<Message>,
}

impl Sender {
    pub fn new() -> Self {
        Sender {
            message_queue: Vec::new(),
        }
    }

    pub fn push(&mut self, message: Message) {
        self.message_queue.push(message);
    }
}
