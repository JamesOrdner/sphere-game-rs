use subsystems::{GameClientSubsystems, GameCoreSubsystems};

mod subsystems;

pub use subsystems::GameSubsystem;

pub struct GameSystem {
    pub core_subsystems: GameCoreSubsystems,
    pub client_subsystems: Option<GameClientSubsystems>,
}

impl GameSystem {
    pub fn create_server() -> GameSystem {
        GameSystem {
            core_subsystems: GameCoreSubsystems::create(),
            client_subsystems: None,
        }
    }

    pub fn create_client() -> GameSystem {
        GameSystem {
            core_subsystems: GameCoreSubsystems::create(),
            client_subsystems: Some(GameClientSubsystems::create()),
        }
    }
}
