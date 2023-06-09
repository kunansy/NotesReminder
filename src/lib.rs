pub mod db {
    use std::collections::HashMap;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;
    use chrono::prelude::*;

    use rand::Rng;
    use sqlx::postgres::PgPool;
    use tokio::join;
    use uuid::Uuid;

    struct RemindInfo {
        repeats_count: i64,
        repeated_at: NaiveDateTime
    }

    struct Note {
        material_id: Uuid,
        title: String,
        authors: String,
        content: String,
        added_at: NaiveDateTime,
        material_status: Option<String>,
    }

    #[derive(Debug)]
    pub struct RemindNote {
        note_id: Uuid,
        content: String,
        added_at: NaiveDateTime,
        notes_count: i64,
        material_title: String,
        material_authors: String,
        material_status: Option<String>,
        material_repeats_count: Option<i64>,
        material_last_repeated_at: Option<NaiveDateTime>
    }

    impl RemindNote {
        pub fn content_html(&self) -> String {
            demark::demark(&self.content)
        }

        pub fn note_id(&self) -> &Uuid {
            &self.note_id
        }

        pub fn repeated_ago(&self) -> String {
            match self.material_last_repeated_at {
                Some(dt) => {
                    let dur = {
                        let dur = Utc::now().naive_utc() - dt;
                        dur.num_days() + 1
                    };

                    let mut s = String::new();
                    if dur / 365 != 0 {
                        let v = (dur / 365) as i32;
                        s.push_str(&format!("{} years", v));
                    }
                    if dur % 365 / 30 != 0 {
                        let v = (dur % 365 / 30) as i32;
                        if !s.is_empty() {
                            s.push_str(", ");
                        }
                        s.push_str(&format!("{} months", v));
                    }
                    if dur % 30 != 0 {
                        let v = (dur % 30) as i32;
                        if !s.is_empty() {
                            s.push_str(", ");
                        }
                        s.push_str(&format!("{} days", v));
                    }
                    s.push_str(" ago");

                    s
                },
                None => "-".to_string()
            }
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
                // it should be unreachable
                None => "undefined"
            };
            let tracker_url = std::env::var("TRACKER_URL")
                .unwrap_or("http://tracker.lan".to_string());

            // don't write time when it not set
            let added_at = {
                let dt = self.added_at;
                if dt.hour() + dt.minute() + dt.second() == 0 {
                    dt.format("%Y-%m-%d").to_string()
                } else {
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            };
            let link = format!("<a href=\"{}/notes/note?note_id={}\">Open</a>",
                               tracker_url, self.note_id);

            let last_material_repeat_info = format!("{}", {
                if self.material_last_repeated_at != None {
                    format!("Repeats count: {}\nLast repeated: {}, {}\n",
                            repeats_count, repeated_at, self.repeated_ago())
                } else {"".to_string()}
            });

            write!(f, "«{}» – {}\n\n{}\n\nMaterial status: {}\nAdded at (UTC): {}\n{}Total notes count: {}\n{}",
                   self.material_title, self.material_authors, self.content_html(), material_status, added_at,
                   last_material_repeat_info, self.notes_count, link)
        }
    }

    pub async fn get_note(pool: &PgPool) -> Result<RemindNote, sqlx::Error> {
        let (notes_count, stat) = join!(get_notes_count(pool), get_remind_statistics(pool));
        let stat = stat?;
        let note_id = get_remind_note_id(&stat);

        let note = get_remind_note(pool, note_id).await?;

        let mut res = RemindNote{
            note_id: *note_id,
            material_title: note.title,
            material_authors: note.authors,
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
                                     note_id: &Uuid,
                                     user_id: i64) -> Result<(), sqlx::Error>{
        let repeated_at = Utc::now().naive_utc();

        sqlx::query!(
            "
            INSERT INTO
                note_repeats_history (repeat_id, note_id, user_id, repeated_at)
            VALUES ($1, $2, $3, $4)
            ",
            create_uuid(), note_id, user_id, repeated_at
        )
            .fetch_all(pool)
            .await?;

        Ok(())
    }

    fn create_uuid() -> Uuid {
        Uuid::new_v4()
    }

    async fn get_notes_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
        log::debug!("Getting notes count");

        let stmt = sqlx::query!("SELECT count(1) FROM notes WHERE not is_deleted;")
            .fetch_one(pool)
            .await?;

        match stmt.count {
            Some(count) => {
                log::debug!("{} notes found", count);
                Ok(count)
            },
            None => panic!("Count not get notes count")
        }
    }

    async fn get_material_repeat_info(pool: &PgPool,
                                      material_id: &Uuid) -> Result<Option<RemindInfo>, sqlx::Error> {
        log::debug!("Getting repeat info for material: {}", material_id);

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
            Some(info) => {
                log::debug!("Repeat info got");
                Ok(Some(RemindInfo{
                    repeats_count: info.count.unwrap(),
                    repeated_at: info.repeated_at
                }))
            },
            None => {
                log::debug!("No repeat info found");
                Ok(None)
            }
        }
    }

    async fn get_remind_statistics(pool: &PgPool) -> Result<HashMap<Uuid, i64>, sqlx::Error> {
        log::debug!("Getting remind statistics");
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
            .map(|row| {(row.note_id, row.count.unwrap_or(0))})
            .collect::<HashMap<Uuid, i64>>();

        log::debug!("Remind statistics got");
        Ok(stat)
    }

    fn get_remind_note_id(stats: &HashMap<Uuid, i64>) -> &Uuid {
        log::debug!("Getting note id to remind");
        if stats.len() == 0 {
            panic!("Empty stats passed");
        }

        let min_f = stats.values().min().unwrap();
        log::debug!("Min frequency is: {}", min_f);

        let min_notes = stats
            .iter()
            .filter(|(_, freq)| freq == &min_f)
            .map(|(note_id, _)| note_id)
            .collect::<Vec<&Uuid>>();

        log::debug!("Total {} notes with it, getting the random one", min_notes.len());
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
                m.material_id, m.title, m.authors, n.content, n.added_at,
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
            let remove_sub_sup = Regex::new(r"<su[bp]>(.*?)</su[bp]>").unwrap();
            let remove_span_sub_sup = Regex::new(r#"<span class="?su[bp]"?>(.*?)</span>"#).unwrap();

            let content = remove_sub_sup.replace_all(content, |r: &Captures| {
                format!("_{}", &r[1])
            });
            remove_span_sub_sup.replace_all(&content, |r: &Captures| {
                format!("_{}", &r[1])
            }).to_string()
        }

        fn dereplace_new_lines(content: &str) -> String {
            content.replace("<br/>", "\n")
                .replace("\r", "")
                .replace("<br>", "\n")
        }
    }
}