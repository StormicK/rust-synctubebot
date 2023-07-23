pub mod sync_tube_model {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Debug, Clone)]
    pub struct CreateRoomResponse {
        pub id: String,
    }

    #[derive(Serialize, Debug)]
    pub struct JoinRoomRequest {
        pub id: String,
        pub preferences: RoomPreferences,
    }

    #[derive(Serialize, Debug)]
    pub struct RoomPreferences {
        pub user: Option<String>,
        pub player: PlayerPreferences,
        pub consent: bool,
    }

    #[derive(Serialize, Debug)]
    pub struct PlayerPreferences {
        pub soundcloud: SoundcloudPreferences,
    }

    #[derive(Serialize, Debug)]
    pub struct SoundcloudPreferences {
        pub volume: f32,
    }
}