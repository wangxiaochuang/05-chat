use futures::StreamExt;
use sqlx::postgres::PgListener;

use crate::AppState;

pub async fn setup_pg_listener(state: AppState) -> anyhow::Result<()> {
    let mut listener = PgListener::connect(&state.config.server.db_url).await?;
    listener.listen("chat_updated").await?;
    listener.listen("chat_message_created").await?;

    let mut stream = listener.into_stream();

    tokio::spawn(async move {
        while let Some(Ok(notification)) = stream.next().await {
            println!("Received notification: {:?}", notification);
        }
    });

    Ok(())
}
