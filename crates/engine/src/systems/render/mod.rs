use subsystems::RenderSubsystems;
use winit::window::Window;

mod subsystems;

pub struct RenderSystem {
    pub subsystems: RenderSubsystems,
}

impl RenderSystem {
    pub fn new(window: Window) -> Self {
        Self {
            subsystems: RenderSubsystems::create(window),
        }
    }
}
