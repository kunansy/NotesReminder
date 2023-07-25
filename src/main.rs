use std::io::Write;
use std::time;

use chrono::Local;
use env_logger::Builder;
use sqlx::PgPool;
use teloxide::{prelude::*, RequestError, types};

use notes_reminder::{db, settings::{load_env, Settings}};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mode = std::env::args()
        .nth(1)
        .expect("Could not get CLI args");

    load_env();
    init_logger();

    let cfg = Settings::parse();
    let pool = db::init_pool(&cfg.db_uri, cfg.db_timeout).await
        .expect("Could not connect to the database");

    let bot = Bot::new(cfg.bot_token)
        .parse_mode(types::ParseMode::Html);

    if mode == "--remind" {
        remind_note(&bot, cfg.chat_id, &pool).await;
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

async fn remind_note<T>(bot: &T, chat_id: i64, pool: &PgPool)
    where T: Requester
{
    let start = time::Instant::now();
    log::info!("Remind a note");

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

    let exec_time = start.elapsed();
    log::info!("Note reminded for {:?}", exec_time);
}

async fn answer<T>(bot: &T, msg: &Message, pool: &PgPool, chat_id: i64) -> Result<(), RequestError>
    where T: Requester
{
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
            remind_note(bot, chat_id, &pool).await;
        },
        _ => {
           bot.send_message(ChatId(chat_id), "Command not found").await
               .expect("Error sending note");
        }
    }
    Ok(())
}

fn init_logger() {
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{}\t[{}] [{}:{}]\t{}",
                     record.level(),
                     Local::now().format("%Y-%m-%d %H:%M:%S.%f"),
                     record.target(),
                     record.line().unwrap_or(1),
                     record.args()
            )
        })
        .parse_default_env()
        .init();
}