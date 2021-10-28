pub type SystemId = u8;

pub struct SystemIdCounter {
    current: SystemId,
}

impl SystemIdCounter {
    pub fn new() -> Self {
        Self { current: 0 }
    }

    pub fn next(&mut self) -> SystemId {
        let ret = self.current;
        self.current += 1;
        ret
    }
}
