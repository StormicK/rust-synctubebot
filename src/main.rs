use reqwest::header::{CONTENT_TYPE, ORIGIN, REFERER};
use reqwest::Error;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::Message;

mod infrastructure;
use infrastructure::client::synctube_client::SyncTubeClient;
use infrastructure::client::synctube_client::SyncTubeClientTrait;

use infrastructure::synctube_client::model::sync_tube_model::{ CreateRoomResponse, JoinRoomRequest, RoomPreferences, PlayerPreferences, SoundcloudPreferences };

struct Room {
    id: String,
    token: String,
}

#[tokio::main]
async fn main() {
    let num_threads = 5;
    let rooms_per_thread = 100;

    let mut join_handles = vec![];

    for _ in 0..num_threads {
        let join_handle = tokio::spawn(create_rooms_task(rooms_per_thread));
        join_handles.push(join_handle);
    }

    // Wait for all threads to finish
    for join_handle in join_handles {
        join_handle.await.unwrap();
    }
}

async fn create_rooms_task(num_rooms: usize) {
    for _ in 0..num_rooms {
        match do_stuff_on_synctube().await {
            Ok(room_url) => println!("Room created: {}", room_url),
            Err(err) => eprintln!("Error creating room: {}", err),
        }
    }
}

async fn do_stuff_on_synctube() -> Result<String, String> {
    let create_room_response: Room = create_room().await.unwrap();
    let room_id: &str = &create_room_response.id;
    let room_token: &str = &create_room_response.token;

    let client = Arc::new(SyncTubeClient::new(room_id, room_token));
    client.connect().await.unwrap();
    client.rename("Rust SyncTube Bot").await.unwrap();
    for _ in 0..9 {
        client
            .add_video("https://www.youtube.com/watch?v=QH2-TGUlwu4")
            .await
            .unwrap();
    }
    client.disconnect().await.unwrap();

    Ok(format!("https://sync-tube.de/room/{}", room_id.to_string()))
}

async fn create_room() -> Result<Room, Error> {
    let client: reqwest::Client = reqwest::Client::new();

    println!("Creating room...");
    let create_room_url: &str = "https://sync-tube.de/api/create";
    let response: reqwest::Response = client.post(create_room_url).send().await?;

    match response.status().is_success() {
        true => println!("Room created successfully!"),
        false => panic!("Room creation failed!"),
    }

    let create_room_response: CreateRoomResponse = response.json().await?;
    let id: &str = &create_room_response.id;
    println!("Room created: {}", id);

    println!("Joining room...");

    let join_room_url: &str = "https://sync-tube.de/api/join";
    let join_room_request: JoinRoomRequest = JoinRoomRequest {
        id: id.to_string(),
        preferences: RoomPreferences {
            user: None,
            player: PlayerPreferences {
                soundcloud: SoundcloudPreferences { volume: 0.2 },
            },
            consent: false,
        },
    };
    let response: reqwest::Response = client
        .post(join_room_url)
        .body(serde_json::to_string(&join_room_request).unwrap())
        .header(CONTENT_TYPE, "application/json")
        .header(ORIGIN, "https://sync-tube.de")
        .header(REFERER, format!("https://sync-tube.de/room/{}", id))
        .send()
        .await?;

    match response.status().is_success() {
        true => println!("Joined room successfully!"),
        false => panic!("Joining room failed!"),
    }

    let s = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(";")
        .collect::<Vec<&str>>()[0]
        .to_string();
    let room: Room = Room {
        id: id.to_string(),
        token: s,
    };

    return Ok(room);
}
