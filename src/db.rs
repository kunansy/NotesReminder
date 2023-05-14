pub mod db {
    use std::collections::HashMap;
    use sqlx::postgres::PgPool;

    struct RemindInfo(i32, chrono::NaiveDateTime);
    pub struct Note { }

    pub async fn get_note(pool: &PgPool) -> Note {
        Note {}
    }

    pub async fn insert_note_history(pool: &PgPool,
                                     note_id: &String,
                                     user_id: i32) {

    }

    async fn get_notes_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
        let row = sqlx::query!("SELECT count(1) FROM notes WHERE not is_deleted;")
            .fetch_one(pool)
            .await?;

        match row.count {
            Some(count) => Ok(count),
            None => panic!("Count not get notes count")
        }
    }

    async fn get_material_repeat_info(pool: &PgPool,
                                      material_id: &String) -> RemindInfo {
        RemindInfo(1, chrono::NaiveDateTime)
    }

    async fn get_remind_statistics(pool: &PgPool) -> HashMap<String, i32> {
        HashMap::new()
    }

    fn get_remind_note_id(stats: HashMap<String, i32>) -> String {
        "".to_string()
    }
}