use std::{convert::Infallible, time::Duration};

use axum::{extract::State, response::Sse, Extension};
use chat_core::User;
use futures::Stream;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::info;

use crate::{notif::AppEvent, AppState};

const CHANNEL_CAPACITY: usize = 256;

pub(crate) async fn sse_handler(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let user_id = user.id as u64;
    let rx = match state.users.get(&user_id) {
        Some(tx) => tx.subscribe(),
        None => {
            let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
            state.users.insert(user_id, tx);
            rx
        }
    };

    info!("User {} subscribed", user_id);

    let stream = BroadcastStream::new(rx).filter_map(|v| v.ok()).map(|v| {
        let name = match v.as_ref() {
            AppEvent::NewChat(_) => "NewChat",
            AppEvent::AddToChat(_) => "AddToChat",
            AppEvent::RemoveFromChat(_) => "RemoveFromChat",
            AppEvent::NewMessage(_) => "NewMessage",
        };
        let v = serde_json::to_string(&v).expect("Failed to serialize event");
        // sse event name
        Ok(axum::response::sse::Event::default().data(v).event(name))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
