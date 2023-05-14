use std::time;
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;

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
        .unwrap_or("10".to_string()).parse().unwrap();
    let timeout = time::Duration::from_secs(timeout);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(timeout)
        .acquire_timeout(timeout)
        .connect(&url).await?;

    if mode == "--remind" {
        let note = db::db::get_note(&pool).await?;
        // send note
        println!("{}", note);
        db::db::insert_note_history(&pool, note.note_id(), 1).await?;
    }

    Ok(())
}