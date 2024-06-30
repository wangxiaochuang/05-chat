use std::{io::Cursor, net::SocketAddr, time::Duration};

use anyhow::Result;
use chat_core::{Chat, ChatType, Message};
use chat_server::test_util;
use futures::StreamExt;
use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use serde_json::json;
use tokio::{net::TcpListener, time::sleep};

struct ChatServer {
    addr: SocketAddr,
    token: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct AuthToken {
    token: String,
}

impl ChatServer {
    async fn try_new(state: chat_server::AppState) -> Result<Self> {
        let app = chat_server::get_router(state.clone()).await?;
        let listener = TcpListener::bind(format!("0.0.0.0:{}", state.config.server.port)).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        let client = reqwest::Client::new();

        let mut ret = Self {
            addr,
            client,
            token: "".to_string(),
        };

        ret.token = ret.signin().await?;

        Ok(ret)
    }

    async fn signin(&self) -> Result<String> {
        let resp = self
            .client
            .post(format!("http://{}/api/signin", self.addr))
            .json(&json!({"email": "jack1@gmail.com", "password": "Hunter48"}))
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::OK);
        let auth: AuthToken = resp.json().await?;
        Ok(auth.token)
    }
    async fn create_chat(&self) -> Result<Chat> {
        let resp = self
            .client
            .post(format!("http://{}/api/chats", self.addr))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&json!({"name": "test", "members": [1, 2], "public": false}))
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let chat: Chat = resp.json().await?;
        assert_eq!(chat.members, vec![1, 2]);
        assert_eq!(chat.r#type, ChatType::PrivateChannel);
        Ok(chat)
    }

    async fn create_message(&self, chat_id: u64) -> Result<Message> {
        let data = include_bytes!("../Cargo.toml");
        let files = Part::bytes(data)
            .file_name("Cargo.toml")
            .mime_str("text/plain")?;
        let form = Form::new().part("files", files);
        let resp = self
            .client
            .post(format!("http://{}/api/upload", self.addr))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::OK);
        let urls: Vec<String> = resp.json().await?;
        let resp = self
            .client
            .post(format!("http://{}/api/chats/{}", self.addr, chat_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&json!({"content": "hello", "files": urls}))
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let message: Message = resp.json().await?;
        assert_eq!(message.content, "hello");
        assert_eq!(message.files, urls);
        assert_eq!(message.sender_id, 1);
        assert_eq!(message.chat_id, chat_id as i64);
        Ok(message)
    }
}

struct NotifyServer;

impl NotifyServer {
    async fn new<R: std::io::Read>(reader: R, db_url: &str, token: &str) -> Result<Self> {
        let mut config = notify_server::config::AppConfig::load_from_reader(reader)?;
        let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server.port)).await?;
        config.server.db_url = db_url.to_string();
        let app = notify_server::get_router(config).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        let mut es = EventSource::get(format!("http://{}/events?token={}", addr, token));
        tokio::spawn(async move {
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => println!("Connection Open!"),
                    Ok(Event::Message(message)) => match message.event.as_str() {
                        "NewChat" => {
                            let chat: Chat = serde_json::from_str(&message.data).unwrap();
                            assert_eq!(chat.name.as_ref().unwrap(), "test");
                            assert_eq!(chat.members, vec![1, 2]);
                            assert_eq!(chat.r#type, ChatType::PrivateChannel);
                            println!("xxxxxxxxx new chat xxxxxxx");
                        }

                        "NewMessage" => {
                            let msg: Message = serde_json::from_str(&message.data).unwrap();
                            assert_eq!(msg.content, "hello");
                            assert_eq!(msg.files.len(), 1);
                            assert_eq!(msg.sender_id, 1);
                            println!("xxxxxxxx newmessage xxxxxxxx");
                        }
                        _ => {
                            panic!("unexpected event: {:?}", message);
                        }
                    },
                    Err(reqwest_eventsource::Error::StreamEnded) => {}
                    Err(err) => {
                        println!("Error: {}", err);
                        es.close();
                    }
                }
            }
        });

        Ok(Self {})
    }
}

const TEST_APP_YAML: &str = r#"
server:
  port: 0
  db_url: postgres://postgres:postgres@localhost:5432/chat
  base_dir: /tmp/chat_server
auth:
  sk: |
    -----BEGIN PRIVATE KEY-----
    MC4CAQAwBQYDK2VwBCIEIJL4hlV1fEGZHFLkhQ99g7MwDwJ+DwXfYZv18fLcj07y
    -----END PRIVATE KEY-----
  pk: |
    -----BEGIN PUBLIC KEY-----
    MCowBQYDK2VwAyEA9Q0GlRpk0eQY/35d414jJ9l6k5xH1SDKCQwg6z/lTmQ=
    -----END PUBLIC KEY-----"#;

const TEST_NOTIFY_YAML: &str = r#"
server:
  port: 6687
  db_url: postgres://postgres:postgres@localhost:5432/chat
auth:
  pk: |
    -----BEGIN PUBLIC KEY-----
    MCowBQYDK2VwAyEA9Q0GlRpk0eQY/35d414jJ9l6k5xH1SDKCQwg6z/lTmQ=
    -----END PUBLIC KEY-----"#;

#[tokio::test]
async fn chat_server_should_work() -> Result<()> {
    let chat_server_config_reader = std::io::BufReader::new(Cursor::new(TEST_APP_YAML.as_bytes()));
    let (state, tdb) =
        test_util::get_test_state_and_pg_from_config_reader(chat_server_config_reader).await?;
    let chat_server = ChatServer::try_new(state).await?;
    let db_url = tdb.url();
    let notify_server_config_reader =
        std::io::BufReader::new(Cursor::new(TEST_NOTIFY_YAML.as_bytes()));
    NotifyServer::new(notify_server_config_reader, &db_url, &chat_server.token).await?;
    let chat = chat_server.create_chat().await?;
    let _message = chat_server.create_message(chat.id as _).await?;
    sleep(Duration::from_secs(1)).await;
    Ok(())
}
