use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use teloxide::{prelude::*, RequestError, types};

use notes_reminder::{db, settings};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mode = std::env::args()
        .nth(1)
        .expect("Could not get CLI args");

    settings::load_env();
    env_logger::init();

    let cfg = settings::Settings::parse();

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
