use std::time;
use dotenv::dotenv;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use teloxide::{prelude::*, types};

mod db;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mode = std::env::args()
        .nth(1)
        .expect("Could not get CLI args");

    // upload .env to env
    dotenv().ok();
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL not found");
    let timeout = std::env::var("DATABASE_TIMEOUT")
        .unwrap_or("10".to_string())
        .parse().expect("DATABASE_TIMEOUT should be int");
    let chat_id: i64 = std::env::var("TG_BOT_USER_ID")
        .expect("TG_BOT_USER_ID not found")
        .parse().expect("User od should be int");
    let timeout = time::Duration::from_secs(timeout);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(timeout)
        .acquire_timeout(timeout)
        .connect(&url).await
        .map_err(|e| e.to_string())?;

    let bot = Bot::from_env()
        .parse_mode(types::ParseMode::Html);

    if mode == "--remind" {
        log::info!("Remind a note");
        send_note(bot, chat_id, &pool).await;
        log::info!("Note reminded");
    } else if mode == "--start" {
        log::info!("Start the bot");

        teloxide::repl(bot, move |bot: Bot, msg: Message| {
            let ChatId(id) = msg.chat.id;
            if chat_id != id {
                log::warn!("Access denied for user: '{}'", id);
            }

            let pool = pool.clone();

            async move {
                send_note(bot, chat_id, &pool).await;
                Ok(())
            }
        }).await;
    } else {
        panic!("Invalid mode passed: {}", mode);
    }

    Ok(())
}

async fn send_note(bot: impl Requester, chat_id: i64, pool: &PgPool) {
    log::info!("Getting a note");
    let note = db::db::get_note(&pool).await
        .expect("Error getting note");
    log::info!("Note got: '{}'", note.note_id());

    log::info!("Sending message to the bot");
    bot.send_message(ChatId(chat_id), &note.to_string()).await
        .expect("Error sending note");
    log::info!("Message sent");

    log::info!("Inserting repeat history");
    db::db::insert_note_history(&pool, note.note_id(), chat_id)
        .await.expect("Error inserting note history");
    log::info!("History inserted");
}