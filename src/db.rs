pub mod db {
    use std::collections::HashMap;
    use std::str::FromStr;

    use rand::Rng;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    struct RemindInfo {
        repeats_count: i64,
        repeated_at: chrono::NaiveDateTime
    }

    struct Note {
        note_id: Uuid,
        material_id: Uuid,
        title: String,
        authors: String,
        content: String,
        added_at: chrono::NaiveDateTime
    }

    pub async fn get_note(pool: &PgPool) -> Note {
        Note {}
    }

    pub async fn insert_note_history(pool: &PgPool,
                                     note_id: &String,
                                     user_id: i64) -> Result<(), sqlx::Error>{
        let note_id = Uuid::from_str(note_id)
            .expect("Invalid note_id");

        sqlx::query!(
            "
            INSERT INTO
                note_repeats_history (repeat_id, note_id, user_id)
            VALUES ($1, $2, $3)
            ",
            create_uuid(), note_id, user_id
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
                                      material_id: &String) -> Result<RemindInfo, sqlx::Error> {
        let material_id = Uuid::from_str(material_id)
            .expect("Invalid material_id");

        let info = sqlx::query!(
            "
            SELECT repeated_at, COUNT(1) OVER (PARTITION BY material_id)
            FROM repeats
            WHERE material_id = $1
            ORDER BY repeated_at DESC
            LIMIT 1;
            ",
            material_id
        )
            .fetch_one(pool)
            .await?;

        Ok(RemindInfo {
            repeats_count: info.count.unwrap(),
            repeated_at: info.repeated_at
        })
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

    async fn get_remind_note(pool: &PgPool, note_id: &Uuid) -> Result<Note, sqlx::Error> {
        sqlx::query_as!(
            Note,
            "
            SELECT
                n.note_id, m.material_id, m.title, m.authors, n.content, n.added_at
            FROM notes n
            JOIN materials m USING(material_id)
            JOIN statuses s USING(material_id)
            WHERE n.note_id = $1 AND NOT n.is_deleted;
            ",
            note_id
        )
            .fetch_one(pool)
            .await
    }
}