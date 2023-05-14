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

    async fn get_remind_statistics(pool: &PgPool) -> Result<HashMap<String, i64>, sqlx::Error> {
        let stat = sqlx::query!(
            "
            WITH stats AS (
                SELECT note_id, count(1)
                FROM note_repeats_history
                GROUP BY note_id
            )
            SELECT
                n.note_id AS note_id,
                COALESCE(s.count, 0) AS count
            FROM notes n
            LEFT JOIN stats s USING(note_id)
            WHERE NOT n.is_deleted;
            "
        )
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| {(row.note_id.to_string(), row.count.unwrap())})
            .collect::<HashMap<String, i64>>();

        Ok(stat)
    }

    fn get_remind_note_id(stats: HashMap<String, i32>) -> String {
        "".to_string()
    }
}