pub mod db {
    use std::fmt::{Display, Formatter};
    use std::time;
    use chrono::prelude::*;

    use sqlx::postgres::{PgPool, PgPoolOptions};
    use uuid::Uuid;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
    #[sqlx(type_name = "materialtypesenum", rename_all = "lowercase")]
    enum MaterialTypes {
        Book,
        Article,
        Lecture,
        Course
    }

    impl MaterialTypes {
        pub fn as_chapter(&self) -> &'static str {
            match self {
                MaterialTypes::Book | MaterialTypes::Article => "Chapter",
                MaterialTypes::Lecture | MaterialTypes::Course => "Part",
            }
        }

        pub fn as_page(&self) -> &'static str {
            match self {
                MaterialTypes::Book | MaterialTypes::Article => "Page",
                MaterialTypes::Lecture => "Minute",
                MaterialTypes::Course => "Lecture",
            }
        }
    }

    #[derive(Debug)]
    pub struct RemindNote {
        note_id: Uuid,
        content: String,
        chapter: i32,
        page: i32,
        added_at: NaiveDateTime,
        notes_count: i64,
        material_title: Option<String>,
        material_authors: Option<String>,
        material_type: Option<MaterialTypes>,
        material_pages: i32,
        material_status: String,
        material_repeats_count: Option<i64>,
        material_last_repeated_at: Option<NaiveDateTime>
    }

    impl RemindNote {
        pub fn material_title(&self) -> &str {
            match &self.material_title {
                Some(v) => v,
                None => &""
            }
        }

        pub fn material_authors(&self) -> &str {
            match &self.material_authors {
                Some(v) => v,
                None => &""
            }
        }

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

        pub fn get_url(&self, tracker_url: &str) -> String {
            format!("{}/notes/note?note_id={}", tracker_url, self.note_id)
        }

        pub fn has_material(&self) -> bool {
            self.material_type.is_some()
        }

        pub fn has_material_repeat(&self) -> bool {
            self.material_last_repeated_at.is_some()
        }
    }

    impl Display for RemindNote {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let repeats_count = self.material_repeats_count.unwrap_or(0);
            let repeated_at = match self.material_last_repeated_at {
                Some(v) => v.format("%Y-%m-%d").to_string(),
                None => "-".to_string()
            };

            // don't write time when it's not set
            let added_at = {
                let dt = self.added_at;
                if dt.hour() + dt.minute() + dt.second() == 0 {
                    dt.format("%Y-%m-%d")
                } else {
                    dt.format("%Y-%m-%d %H:%M:%S")
                }
            };

            let mut rows = Vec::with_capacity(10);

            if self.has_material() {
                let material_info = format!(
                    "«{}» – {}\n", self.material_title(), self.material_authors());
                rows.push(material_info)
            }

            rows.push(self.content_html());
            rows.push(String::new());

            if self.has_material() {
                let material_type = self.material_type.as_ref().unwrap();

                rows.push(format!("{}: {}", material_type.as_chapter(), self.chapter));
                rows.push(format!("{}: {}/{}", material_type.as_page(), self.page, self.material_pages));
                rows.push(format!("Material status: {}", self.material_status));
            }
            rows.push(format!("Added at (UTC): {}", added_at));

            if self.has_material_repeat() {
                rows.push(format!("Repeats count: {}", repeats_count));
                rows.push(format!("Last repeated: {}, {}", repeated_at, self.repeated_ago()))
            }
            rows.push(format!("Total notes count: {}", self.notes_count));

            write!(f, "{}", rows.join("\n"))
        }
    }

    pub async fn get_note(pool: &PgPool) -> Result<RemindNote, sqlx::Error> {
        // TODO: use a matview?
        let stmt = sqlx::query!(r#"
            WITH repeated_notes_freq AS (
                SELECT note_id, count(1)
                FROM note_repeats_history
                GROUP BY note_id
            ),
            all_notes_freq AS (
                SELECT
                    n.material_id,
                    n.note_id AS note_id,
                    COALESCE(s.count, 0) AS count,
                    COUNT(1) OVER () AS total
                FROM notes n
                LEFT JOIN repeated_notes_freq s USING(note_id)
                WHERE NOT n.is_deleted
            ),
            min_freq AS (
                SELECT count
                FROM all_notes_freq
                ORDER BY count
                LIMIT 1
            ),
            sample_notes AS (
                SELECT
                    n.note_id,
                    n.material_id,
                    n.page,
                    n.chapter,
                    n.title,
                    n.content,
                    n.added_at,
                    f.total AS total_notes_count,
                    -- m.total AS total_freq_count,
                    m.count AS min_repeat_freq
                FROM all_notes_freq f
                JOIN min_freq m ON f.count = m.count
                JOIN notes n ON f.note_id = n.note_id
            ),
            materials_cte AS (
                SELECT DISTINCT material_id FROM all_notes_freq
            ),
            last_repeat AS (
                SELECT
                    r.material_id,
                    MAX(r.repeated_at) AS repeated_at,
                    COUNT(1) AS count
                FROM materials_cte n
                JOIN repeats r USING(material_id)
                GROUP BY r.material_id
            )
            SELECT
                n.note_id,
                m.title AS "material_title?",
                m.authors AS "material_authors?",
                m.material_type AS "material_type?: MaterialTypes",
                n.content,
                n.added_at,
                n.chapter,
                n.page,
                m.pages AS "material_pages?",
                n.total_notes_count AS "total_notes_count!",
                -- count of notes to repeat with this frequency
                -- n.total_freq_count AS "total_freq_count!",
                -- min frequency of notes repeating
                n.min_repeat_freq AS "min_repeat_freq?",
                CASE
                    -- in this case the note have no material
                    WHEN m IS NULL THEN 'completed'
                    WHEN s IS NULL THEN 'queue'
                    WHEN s.completed_at IS NULL THEN 'reading'
                    ELSE 'completed'
                END AS "material_status!",
                r.repeated_at AS "material_last_repeated_at?",
                r.count AS "material_repeats_count?"
            FROM
                sample_notes n
            LEFT JOIN materials m on n.material_id = m.material_id
            LEFT JOIN statuses s on s.material_id = m.material_id
            LEFT JOIN last_repeat r on r.material_id = s.material_id
            ORDER BY random()
            LIMIT 1
        "#)
            .fetch_one(pool)
            .await?;

        // TODO
        log::info!("Min repeat freq {:?}, notes with it --, choose the random one", stmt.min_repeat_freq);

        Ok(RemindNote {
            note_id: stmt.note_id,
            content: stmt.content,
            page: stmt.page,
            chapter: stmt.chapter,
            added_at: stmt.added_at,
            notes_count: stmt.total_notes_count,
            material_title: stmt.material_title,
            material_authors: stmt.material_authors,
            material_type: stmt.material_type,
            material_pages: stmt.material_pages.unwrap_or(0),
            material_status: stmt.material_status,
            material_repeats_count: stmt.material_repeats_count,
            material_last_repeated_at: stmt.material_last_repeated_at,
        })
    }

    pub async fn insert_note_history(pool: &PgPool,
                                     note_id: &Uuid,
                                     user_id: i64) -> Result<(), sqlx::Error>{
        let repeated_at = Utc::now().naive_utc();

        sqlx::query!(
            "
            INSERT INTO
                note_repeats_history (repeat_id, note_id, user_id, repeated_at)
            VALUES ($1::uuid, $2::uuid, $3::bigint, $4::timestamp)
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

    pub async fn init_pool(uri: &str, timeout: time::Duration) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new()
            .max_connections(5)
            .idle_timeout(timeout)
            .acquire_timeout(timeout)
            .connect(uri).await
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

pub mod tracker_api {
    use hyper::{Client, body::Buf, http::uri};
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct RepeatItem {
        material_id: String,
        title: String,
        pages: i32,
        material_type: serde_json::Value,
        pub is_outlined: bool,
        notes_count: i32,
        repeats_count: i32,
        completed_at: String,
        last_repeated_at: Option<String>,
        priority_days: i32,
        pub priority_months: i32
    }

    pub async fn get_repeat_queue(tracker_url: &str) -> Result<Vec<RepeatItem>, String> {
        log::debug!("Getting repeat queue");
        let client = Client::new();

        let url = format!("{}/materials/repeat-queue", tracker_url).parse()
            .map_err(|e: uri::InvalidUri| e.to_string())?;

        let resp = client.get(url)
            .await.map_err(|e| e.to_string())?;
        let body = hyper::body::aggregate(resp)
            .await.map_err(|e| e.to_string())?;

        let json: Vec<RepeatItem> = serde_json::from_reader(body.reader())
            .map_err(|e| e.to_string())?;

        log::debug!("{} queue items found", &json.len());
        Ok(json)
    }
}

pub mod settings {
    use std::{fs, time};

    pub struct Settings {
        pub db_uri: String,
        pub db_timeout: time::Duration,
        pub chat_id: i64,
        pub bot_token: String,
        pub tracker_url: String,
        pub tracker_web_url: String,
    }

    impl Settings {
        pub fn parse<'a>() -> Result<Self, &'a str> {
            log::debug!("Parse settings");

            let db_uri = std::env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL not found")?;

            let timeout = std::env::var("DATABASE_TIMEOUT")
                .unwrap_or("10".to_string())
                .parse()
                .map_err(|_| "DATABASE_TIMEOUT should be int")?;
            let db_timeout = time::Duration::from_secs(timeout);

            let bot_token = std::env::var("TG_BOT_TOKEN")
                .map_err(|_| "TG_BOT_TOKEN not found")?;

            let tracker_url = std::env::var("TRACKER_URL")
                .map_or(None, |mut v| {
                    if v.ends_with('/') {
                        v.pop();
                    }
                    Some(v)
                })
                .ok_or("TRACKER_URL not found")?;

            let tracker_web_url = std::env::var("TRACKER_WEB_URL")
                .map_or(None, |mut v| {
                    if v.ends_with('/') {
                        v.pop();
                    }
                    Some(v)
                })
                .unwrap_or(tracker_url.clone());

            let chat_id: i64 = std::env::var("TG_BOT_USER_ID")
                .map_err(|_| "TG_BOT_USER_ID not found")?
                .parse()
                .map_err(|_| "User id should be int")?;

            log::debug!("Settings parsed");
            Ok(Self { db_uri, db_timeout, bot_token, chat_id, tracker_url, tracker_web_url })
        }
    }

    /// Load .env file to env.
    ///
    /// # Errors
    ///
    /// Warn if it could not read file, don't panic.
    pub fn load_env() {
        let env = match fs::read_to_string(".env") {
            Ok(content) => content,
            Err(e) => {
                log::warn!("Error reading .env file: {}", e);
                return;
            }
        };

        for line in env.lines() {
            if line.is_empty() {
                continue;
            }
            let (name, value) = line.split_once("=").unwrap();
            // there might be spaces around the '=', so trim the strings
            std::env::set_var(name.trim(), value.trim());
        }
    }
}
