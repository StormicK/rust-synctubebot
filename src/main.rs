use reqwest::header::{CONTENT_TYPE, ORIGIN, REFERER};
use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::format;
use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, tungstenite::handshake::client::generate_key};
use http::request::Request;

#[derive(Deserialize, Debug, Clone)]
struct CreateRoomResponse {
    id: String,
}

#[derive(Serialize, Debug)]
struct JoinRoomRequest {
    id: String,
    preferences: RoomPreferences,
}

#[derive(Serialize, Debug)]
struct RoomPreferences {
    user: Option<String>,
    player: PlayerPreferences,
    consent: bool,
}

#[derive(Serialize, Debug)]
struct PlayerPreferences {
    soundcloud: SoundcloudPreferences,
}

#[derive(Serialize, Debug)]
struct SoundcloudPreferences {
    volume: f32,
}

struct Room {
    id: String,
    token: String,
}

#[tokio::main]
async fn main() {
    let create_room_response: Room = create_room().await.unwrap();
    let room_id: &str = &create_room_response.id;
    let room_token: &str = &create_room_response.token;

    let connect_addr = format!("wss://sync-tube.de/ws/{}/ewB9AA==", room_id);

    let request = Request::builder()
        .uri(&connect_addr)
        .method("GET")
        .header("Host", "sync-tube.de")
        .header("Origin", "https://sync-tube.de/")
        .header("Cookie", room_token)
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .body(())
        .unwrap();

    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx)); 

    let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout().write_all(&data).await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
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
