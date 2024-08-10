mod tracker_api;
use std::{io::Write, process::exit, thread, time};

use chrono::Local;
use env_logger::Builder;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use sqlx::PgPool;
use teloxide::{prelude::*, types, types::{InlineKeyboardButton, InlineKeyboardMarkup}};

use notes_reminder::{db, settings::{load_env, Settings}};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut signals = Signals::new(&[SIGTERM, SIGQUIT, SIGINT])
        .map_err(|e| e.to_string())?;

    thread::spawn(move || {
        for sig in signals.forever() {
            log::info!("Received a signal '{:?}', terminating", sig);
            exit(0);
        }
    });
    let mode = std::env::args()
        .nth(1)
        .ok_or("Could not get CLI args")?;

    load_env();
    init_logger();

    let cfg = Settings::parse()?;
    let pool = db::init_pool(&cfg.db_uri, cfg.db_timeout).await
        .map_err(|_| "Could not connect to the database")?;

    let bot = Bot::new(cfg.bot_token.clone())
        .parse_mode(types::ParseMode::Html);

    if mode == "--remind" {
        remind_note(&bot, &cfg, &pool).await;
    } else if mode == "--start" {
        log::info!("Start the bot");

        teloxide::repl(bot.clone(), move |msg: Message| {
            // here it must be parseable
            let cfg = Settings::parse().unwrap();
            let bot = bot.clone();
            let pool = pool.clone();

            async move {
                answer(&bot, &msg, &pool, &cfg).await
            }
        }).await;

    } else if mode == "--repeat" {
        remind_repeat(&bot, cfg.chat_id, &cfg.tracker_url, &cfg.tracker_web_url)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        return Err(format!("Invalid mode passed: {}", mode));
    }

    Ok(())
}

async fn remind_note<T>(bot: &T, cfg: &Settings, pool: &PgPool)
    where T: Requester
{
    let start = time::Instant::now();
    log::info!("Remind a note");

    log::info!("Getting a note");
    let note = db::get_note(&pool).await
        .expect("Error getting note");
    log::info!("Note got: '{}'", note.note_id());

    log::info!("Sending message to the bot");

    let keyboard = {
        let url = note.get_url(&cfg.tracker_web_url).parse().unwrap();
        let open_button = InlineKeyboardButton::url("Open".to_string(), url);
        InlineKeyboardMarkup::default().append_row(vec![open_button])
    };

    // TODO: process API, timeout errors
    bot.send_message(ChatId(cfg.chat_id), &note.to_string())
        .reply_markup(keyboard)
        .await
        .expect("Error sending note");
    log::info!("Message sent");

    log::info!("Inserting repeat history");
    db::insert_note_history(&pool, note.note_id(), cfg.chat_id)
        .await.expect("Error inserting note history");
    log::info!("History inserted");

    log::info!("Refresh repeat notes view");
    db::refresh_repeat_notes_view(&pool)
        .await.expect("Error refreshing the view");
    log::info!("View refreshed");

    let exec_time = start.elapsed();
    log::info!("Note reminded for {:?}", exec_time);
}

async fn answer<T>(bot: &T,
                   msg: &Message,
                   pool: &PgPool,
                   cfg: &Settings) -> Result<(), T::Err>
    where T: Requester
{
    let ChatId(id) = msg.chat.id;
    if cfg.chat_id != id {
        log::warn!("Access denied for user: '{}'", id);
        return Ok(());
    }

    match msg.text() {
        Some("/start") => {
            log::info!("[{}]: User starts the bot", cfg.chat_id);
            bot.send_message(ChatId(cfg.chat_id), "/remind to remind the note").await?;
        },
        Some("/remind") => {
            log::info!("[{}]: User reminds a note", cfg.chat_id);
            remind_note(bot, &cfg, &pool).await;
        },
        Some("/repeat") => {
            log::info!("Remind to repeat");
            remind_repeat(bot, cfg.chat_id, &cfg.tracker_url, &cfg.tracker_web_url).await?;
        },
        _ => {
           bot.send_message(ChatId(cfg.chat_id), "Command not found").await?;
        }
    }
    Ok(())
}

async fn remind_repeat<T>(bot: &T, chat_id: i64, tracker_url: &str, tracker_web_url: &str) -> Result<(), T::Err>
    where T: Requester
{
    let repeat_q = tracker_api::get_repeat_queue(tracker_url)
        .await.expect("Could not get repeat queue");

    if repeat_q.is_empty() {
        log::info!("There's nothing to remind, terminating");
        return Ok(());
    }

    let msg = format!("You have {} materials to repeat, including {} outlined. Max priority is {}.\n\n\
                             It's time to <a href=\"{}/materials/repeat-view\">repeat</a>!",
                      repeat_q.len(),
                      repeat_q.iter().filter(|&r| r.is_outlined).count(),
                      repeat_q.iter().map(|r| r.priority_months).max().unwrap_or(0),
                      tracker_web_url);

    bot.send_message(ChatId(chat_id), msg).await?;

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
