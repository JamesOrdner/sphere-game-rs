use subsystems::RenderSubsystems;
use winit::window::Window;

mod subsystems;

pub struct RenderSystem {
    pub subsystems: RenderSubsystems,
}

pub fn create_system(window: Window) -> RenderSystem {
    RenderSystem {
        subsystems: RenderSubsystems::create(window),
    }
}
