use std::time;
use dotenv::dotenv;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use teloxide::prelude::*;

mod db;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error>{
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
        .connect(&url).await?;

    let bot = Bot::from_env();

    if mode == "--remind" {
        send_note(bot, chat_id, &pool).await;
    }
        let note = db::db::get_note(&pool).await?;
        // send note
        println!("{}", note);
        db::db::insert_note_history(&pool, note.note_id(), 1).await?;
    }

    Ok(())
}

async fn send_note(bot: Bot, chat_id: i64, pool: &PgPool) {
    let note = db::db::get_note(&pool).await
        .expect("Error getting note");

    bot.send_message(ChatId(chat_id), &note.to_string()).await
        .expect("Error sending note");

    db::db::insert_note_history(&pool, note.note_id(), chat_id)
        .await.expect("Error inserting note history");
}