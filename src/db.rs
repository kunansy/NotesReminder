pub mod db {
    use std::collections::HashMap;
    use std::str::FromStr;

    use rand::Rng;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    struct RemindInfo(i32, chrono::NaiveDateTime);
    pub struct Note { }

    pub async fn get_note(pool: &PgPool) -> Note {
        Note {}
    }

    pub async fn insert_note_history(pool: &PgPool,
                                     note_id: &String,
                                     user_id: i64) -> Result<(), sqlx::Error>{
        let repeat_id = create_uuid();
        let note_id = Uuid::from_str(note_id)
            .expect("Invalid note_id");

        sqlx::query!(
            "
            INSERT INTO
                note_repeats_history (repeat_id, note_id, user_id)
            VALUES ($1, $2, $3)
            ",
            repeat_id, note_id, user_id
        )
            .fetch_all(pool)
            .await?;

        Ok(())
    }

    fn create_uuid() -> Uuid {
        Uuid::new_v4()
    }

    async fn get_notes_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
        let stmt = sqlx::query!("SELECT count(1) FROM notes WHERE not is_deleted;")
            .fetch_one(pool)
            .await?;

        match stmt.count {
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

    fn get_remind_note_id(stats: &HashMap<String, i64>) -> String {
        if stats.len() == 0 {
            panic!("Empty stats passed");
        }

        let min_f = stats.values().min().unwrap();
        let min_notes = stats
            .iter()
            .filter(|(_, freq)| freq == &min_f)
            .map(|(note_id, _)| note_id)
            .collect::<Vec<&String>>();

        let index = rand::thread_rng().gen_range(0..min_notes.len());
        let &note_id = min_notes.get(index)
            .expect("Could not get list element");

        note_id.clone()
    }
}