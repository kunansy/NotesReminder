use std::{fs, time};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use teloxide::{prelude::*, RequestError, types};

use notes_reminder::db;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mode = std::env::args()
        .nth(1)
        .expect("Could not get CLI args");

    load_env();
    env_logger::init();

    let cfg = Settings::parse();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(cfg.db_timeout)
        .acquire_timeout(cfg.db_timeout)
        .connect(&cfg.db_uri).await
        .expect("Could not connect to the database");

    let bot = Bot::new(cfg.bot_token)
        .parse_mode(types::ParseMode::Html);

    if mode == "--remind" {
        log::info!("Remind a note");
        send_note(bot, cfg.chat_id, &pool).await;
        log::info!("Note reminded");
    } else if mode == "--start" {
        log::info!("Start the bot");

        teloxide::repl(bot.clone(), move |msg: Message| {
            let bot = bot.clone();
            let pool = pool.clone();

            async move {
                answer(&bot, &msg, &pool, cfg.chat_id).await
            }
        }).await;
    } else {
        panic!("Invalid mode passed: {}", mode);
    }

    Ok(())
}

async fn send_note(bot: impl Requester, chat_id: i64, pool: &PgPool) {
    log::info!("Getting a note");
    let note = db::get_note(&pool).await
        .expect("Error getting note");
    log::info!("Note got: '{}'", note.note_id());

    log::info!("Sending message to the bot");
    // TODO: process API, timeout errors
    bot.send_message(ChatId(chat_id), &note.to_string()).await
        .expect("Error sending note");
    log::info!("Message sent");

    log::info!("Inserting repeat history");
    db::insert_note_history(&pool, note.note_id(), chat_id)
        .await.expect("Error inserting note history");
    log::info!("History inserted");
}

async fn answer(bot: &impl Requester, msg: &Message, pool: &PgPool, chat_id: i64) -> Result<(), RequestError> {
    let ChatId(id) = msg.chat.id;
    if chat_id != id {
        log::warn!("Access denied for user: '{}'", id);
        return Ok(());
    }

    match msg.text() {
        Some("/start") => {
            log::info!("[{}]: User starts the bot", chat_id);
            bot.send_message(ChatId(chat_id), "/remind to remind the note").await
                .expect("Error sending note");
        },
        Some("/remind") => {
            log::info!("[{}]: User reminds a note", chat_id);
            send_note(bot, chat_id, &pool).await;
        },
        _ => {
           bot.send_message(ChatId(chat_id), "Command not found").await
               .expect("Error sending note");
        }
    }
    Ok(())
}

struct Settings {
    db_uri: String,
    db_timeout: time::Duration,
    chat_id: i64,
    bot_token: String
}

impl Settings {
    fn parse() -> Self {
        log::debug!("Parse settings");

        let db_uri = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL not found");
        let timeout = std::env::var("DATABASE_TIMEOUT")
            .unwrap_or("10".to_string())
            .parse().expect("DATABASE_TIMEOUT should be int");
        let bot_token = std::env::var("TG_BOT_TOKEN")
            .expect("TG_BOT_TOKEN not found");
        let chat_id: i64 = std::env::var("TG_BOT_USER_ID")
            .expect("TG_BOT_USER_ID not found")
            .parse().expect("User id should be int");
        let db_timeout = time::Duration::from_secs(timeout);

        log::debug!("Settings parsed");
        Self { db_uri, db_timeout, bot_token, chat_id }
    }
}

/// Load .env file to env.
///
/// # Errors
///
/// Warn if it could not read file, don't panic.
fn load_env() {
    let env = match fs::read_to_string(".env") {
        Ok(content) => content,
        Err(e) => {
            log::warn!("Error reading .env file: {}", e);
            return;
        }
    };

    for line in env.lines() {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once("=").unwrap();
        // there might be spaces around the '=', so trim the strings
        std::env::set_var(name.trim(), value.trim());
    }
}