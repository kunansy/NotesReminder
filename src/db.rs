pub mod db {
    use std::collections::HashMap;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;

    use rand::Rng;
    use sqlx::postgres::PgPool;
    use tokio::join;
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
        added_at: chrono::NaiveDateTime,
        material_status: Option<String>,
    }

    #[derive(Debug)]
    pub struct RemindNote {
        note_id: String,
        title: String,
        authors: String,
        content: String,
        added_at: chrono::NaiveDateTime,
        material_status: Option<String>,
        notes_count: i64,
        material_repeats_count: Option<i64>,
        material_last_repeated_at: Option<chrono::NaiveDateTime>
    }

    impl RemindNote {
        pub fn content_html(&self) -> String {
            demark::demark(&self.content)
        }
    }

    impl Display for RemindNote {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let repeats_count = self.material_repeats_count.unwrap_or(0);
            let repeated_at = match self.material_last_repeated_at {
                Some(v) => v.format("%Y-%m-%d").to_string(),
                None => "-".to_string()
            };
            let material_status = match &self.material_status {
                Some(v) => v,
                None => "undefined"
            };
            let tracker_url = std::env::var("TRACKER_URL")
                .unwrap_or("http://tracker.lan".to_string());

            write!(f, "«{}» – {}\n\n{:?}\nMaterial status: {}\nAdded at: {}\nRepeats count: {}\n\
            Last repeated: {}\nTotal notes count: {}\nOpen: {}/notes/note?note_id={}",
                   self.title, self.authors, self.content_html(), material_status, self.added_at,
                   repeats_count, repeated_at, self.notes_count, tracker_url, self.note_id)
        }
    }

    pub async fn get_note(pool: &PgPool) -> Result<RemindNote, sqlx::Error> {

        let (notes_count, stat) = join!(get_notes_count(pool), get_remind_statistics(pool));
        let stat = stat?;
        let note_id = get_remind_note_id(&stat);

        let note = get_remind_note(pool, note_id).await?;

        let mut res = RemindNote{
            note_id: note_id.to_string(),
            title: note.title,
            authors: note.authors,
            content: note.content,
            added_at: note.added_at,
            material_status: note.material_status,
            notes_count: notes_count?,
            material_repeats_count: None,
            material_last_repeated_at: None
        };
        if let Some(info) = get_material_repeat_info(pool, &note.material_id).await? {
            res.material_repeats_count = Some(info.repeats_count);
            res.material_last_repeated_at = Some(info.repeated_at);
        }

        Ok(res)
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
                                      material_id: &Uuid) -> Result<Option<RemindInfo>, sqlx::Error> {
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
            .fetch_optional(pool)
            .await?;

        match info {
            Some(info) => Ok(Some(RemindInfo{
                repeats_count: info.count.unwrap(),
                repeated_at: info.repeated_at
            })),
            None => Ok(None)
        }
    }

    async fn get_remind_statistics(pool: &PgPool) -> Result<HashMap<Uuid, i64>, sqlx::Error> {
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
            .map(|row| {(row.note_id, row.count.unwrap())})
            .collect::<HashMap<Uuid, i64>>();

        Ok(stat)
    }

    fn get_remind_note_id(stats: &HashMap<Uuid, i64>) -> &Uuid {
        if stats.len() == 0 {
            panic!("Empty stats passed");
        }

        let min_f = stats.values().min().unwrap();
        let min_notes = stats
            .iter()
            .filter(|(_, freq)| freq == &min_f)
            .map(|(note_id, _)| note_id)
            .collect::<Vec<&Uuid>>();

        let index = rand::thread_rng().gen_range(0..min_notes.len());
        let &note_id = min_notes.get(index)
            .expect("Could not get list element");

        note_id
    }

    async fn get_remind_note(pool: &PgPool, note_id: &Uuid) -> Result<Note, sqlx::Error> {
        // TODO: sqlx thinks than CASE might produce None
        sqlx::query_as!(
            Note,
            "
            SELECT
                n.note_id, m.material_id, m.title, m.authors, n.content, n.added_at,
                CASE
                    WHEN s IS NULL THEN 'queue'
                    WHEN s.completed_at IS NULL THEN 'reading'
                    ELSE 'completed'
                END AS material_status
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

    mod demark {
        use regex::{Captures, Regex};

        pub fn demark(content: &str) -> String {
            remove_sub_sup(&dereplace_new_lines(&demark_code(
                &demark_italic(&demark_bold(content))))).to_string()
        }

        fn demark_bold(content: &str) -> String {
            let demark_bold_pattern = Regex::new(r#"<span class="?font-weight-bold"?>(.*?)</span>"#).unwrap();
            demark_bold_pattern.replace_all(content, |r: &Captures| {
                format!("<b>{}</b>", &r[1])
            }).to_string()
        }

        fn demark_italic(content: &str) -> String {
            let demark_italic_pattern = Regex::new(r#"<span class="?font-italic"?>(.*?)</span>"#).unwrap();
            demark_italic_pattern.replace_all(content, |r: &Captures| {
                format!("<i>{}</i>", &r[1])
            }).to_string()
        }

        fn demark_code(content: &str) -> String {
            let demark_code_pattern = Regex::new(r#"<span class="?font-code"?>(.*?)</span>"#).unwrap();
            demark_code_pattern.replace_all(content, |r: &Captures| {
                format!("<code>{}</code>", &r[1])
            }).to_string()
        }

        fn remove_sub_sup(content: &str) -> String {
            let remove_sub_sup = Regex::new(r"<su[bp]>(.*?)</su[pb]>").unwrap();
            let remove_span_sub_sup = Regex::new(r#"<span class="?su[bp]"?>(.*?)</span>"#).unwrap();

            let content = remove_sub_sup.replace_all(content, |r: &Captures| {
                format!("_{}", &r[1])
            });
            remove_span_sub_sup.replace_all(&content, |r: &Captures| {
                format!("_{}", &r[1])
            }).to_string()
        }

        fn dereplace_new_lines(content: &str) -> String {
            content.replace(r"<br/?>", "\n")
        }
    }
}