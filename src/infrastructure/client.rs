pub mod synctube_client {
    use crate::Message;
    use async_trait::async_trait;
    use futures_util::stream::SplitSink;
    use futures_util::stream::SplitStream;
    use futures_util::SinkExt;
    use futures_util::StreamExt;
    use std::sync::Arc;
    use tokio::net::TcpStream;
    use tokio::sync::Mutex;
    use tokio_tungstenite::MaybeTlsStream;
    use tokio_tungstenite::WebSocketStream;
    use tokio_tungstenite::{connect_async, tungstenite::handshake::client::generate_key};

    use http::request::Request;

    #[async_trait]
    pub trait SyncTubeClientTrait: Sync + Send {
        async fn connect(&self) -> Result<(), String>;
        async fn disconnect(&self) -> Result<(), String>;
        async fn add_video(&self, video_id: &str) -> Result<(), String>;
        async fn rename(&self, name: &str) -> Result<(), String>;
        async fn get_playlist_count(&self) -> Result<u32, String>;

        fn get_connected(&self) -> bool;
        fn get_error(&self) -> Option<String>;
    }

    pub struct SyncTubeClient {
        room_id: String,
        room_token: String,
        disconnect: bool,
        write: Arc<Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
        read: Arc<Mutex<Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>>,
    }

    // implementation
    impl SyncTubeClient {
        pub fn new(room_id: &str, room_token: &str) -> Self {
            SyncTubeClient {
                room_id: room_id.to_string(),
                room_token: room_token.to_string(),
                disconnect: false,
                write: Arc::new(Mutex::new(None)),
                read: Arc::new(Mutex::new(None)),
            }
        }

        async fn send_message(&self, message: &str) -> Result<(), String> {
            let mut write = self.write.lock().await;

            while write.is_none() {
                println!("write is none");
                write = self.write.lock().await;
            }

            // Obtain a reference to the SplitSink and clone it
            let write = write.as_mut().unwrap();

            let msg = Message::Text(message.to_string());
            println!("Sending message: {:?}", msg);

            match write.send(msg).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }

        async fn listen(self: Arc<Self>) -> Result<(), String> {
            // make it retry until its able to obtain the lock
            let mut read_mutex = self.read.lock().await;
            while read_mutex.is_none() {
                read_mutex = self.read.lock().await;
            }

            let read = read_mutex.as_mut().unwrap();
            while let Some(msg) = read.next().await {
                let msg = msg.unwrap();
                println!("Received a message: {:?}", msg);
                if self.disconnect {
                    break;
                }
            }

            self.disconnect().await?;
            Ok(())
        }
    }

    #[async_trait]
    impl SyncTubeClientTrait for Arc<SyncTubeClient> {
        // make empty impls
        async fn connect(&self) -> Result<(), String> {
            println!("Connecting to SyncTube server...");
            println!("{}", &self.room_id);
            let request = Request::builder()
                .uri(format!("wss://sync-tube.de/ws/{}/ewB9AA==", &self.room_id))
                .method("GET")
                .header("Host", "sync-tube.de")
                .header("Origin", "https://sync-tube.de/")
                .header("Cookie", &self.room_token)
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", generate_key())
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .body(())
                .unwrap();

            let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");
            println!("WebSocket handshake has been successfully completed");

            // split the ws_stream into read and write and put it into the mutexes
            let (write, read) = ws_stream.split();
            
            *self.write.lock().await = Some(write);
            *self.read.lock().await = Some(read);

            // I want to run listen in a tokio spawn
            // Clone the Arc to ensure it's owned by the spawned task
            let self_clone = Arc::clone(self);

            // Run the listen method in the background using tokio::spawn
            tokio::spawn(async move {
                if let Err(e) = SyncTubeClient::listen(self_clone).await {
                    println!("Error occurred during listening: {:?}", e);
                }
            });

            Ok(())
        }

        async fn disconnect(&self) -> Result<(), String> {
            Ok(())
        }

        async fn add_video(&self, video_id: &str) -> Result<(), String> {
            let event = format!("[30, {{\"src\": \"{}\"}}, 1688789078114]", video_id);
            println!("Sending add_video event: {}", &event);
            self.send_message(&event).await?;

            Ok(())
        }
        
        async fn rename(&self, name: &str) -> Result<(), String> {
            let event = format!("[12, \"{}\", 1688792619084]", name);
            println!("Sending rename event: {}", &event);
            self.send_message(&event).await?;

            Ok(())
        }

        async fn get_playlist_count(&self) -> Result<u32, String> {
            Ok(0)
        }

        fn get_connected(&self) -> bool {
            false
        }

        fn get_error(&self) -> Option<String> {
            None
        }
    }
}
