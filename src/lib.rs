pub mod db {
    use std::fmt::{Display, Formatter};
    use std::time;
    use chrono::prelude::*;

    use sqlx::postgres::{PgPool, PgPoolOptions};
    use sqlx::types::JsonValue;
    use uuid::Uuid;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, sqlx::Type, Deserialize, Serialize)]
    #[sqlx(type_name = "materialtypesenum", rename_all = "lowercase")]
    #[serde(rename_all = "lowercase")]
    pub enum MaterialTypes {
        Book,
        Article,
        Lecture,
        Course,
        Audiobook,
    }

    impl MaterialTypes {
        pub fn as_chapter(&self) -> &'static str {
            match self {
                MaterialTypes::Book | MaterialTypes::Article | MaterialTypes::Audiobook => "Chapter",
                MaterialTypes::Lecture | MaterialTypes::Course => "Part",
            }
        }

        pub fn as_page(&self) -> &'static str {
            match self {
                MaterialTypes::Book | MaterialTypes::Article => "Page",
                MaterialTypes::Lecture | MaterialTypes::Audiobook => "Minute",
                MaterialTypes::Course => "Lecture",
            }
        }
    }

    impl Display for MaterialTypes {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[derive(Debug)]
    pub struct RemindNote {
        note_id: Uuid,
        content: String,
        chapter: String,
        page: i32,
        added_at: NaiveDateTime,
        tags: JsonValue,
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
            let mut d = demark::Demark::from(self.content.clone());
            d.demark()
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

                    let mut s = Vec::with_capacity(4);
                    if dur / 365 != 0 {
                        let v = (dur / 365) as i32;
                        s.push(format!("{} years", v));
                    }
                    if dur % 365 / 30 != 0 {
                        let v = (dur % 365 / 30) as i32;
                        s.push(format!("{} months", v));
                    }
                    if dur % 30 != 0 {
                        let v = (dur % 30) as i32;
                        s.push(format!("{} days", v));
                    }

                    format!("{} ago", s.join(", "))
                },
                None => "-".to_string()
            }
        }

        pub fn tags(&self) -> Vec<String> {
            self.tags.as_array().unwrap().iter().map(|tag| format!("#{}", tag.as_str().unwrap())).collect()
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
            // don't write time when it's not set
            let added_at = {
                let dt = self.added_at;
                if dt.hour() + dt.minute() + dt.second() == 0 {
                    dt.format("%Y-%m-%d")
                } else {
                    dt.format("%Y-%m-%d %H:%M:%S")
                }
            };

            let mut rows = Vec::with_capacity(12);

            if self.has_material() {
                let material_info = {
                    let title = self.material_title.as_ref().unwrap();
                    let authors = self.material_authors.as_ref().unwrap();

                    format!("«{}» – {}\n", title, authors)
                };
                rows.push(material_info)
            }

            rows.push(self.content_html());
            rows.push(String::new());

            let tags = self.tags();
            if tags.len() > 0 {
                rows.push(format!("{}", tags.join(" ")));
                rows.push(String::new());
            }

            if self.has_material() {
                let material_type = self.material_type.as_ref().unwrap();

                if self.chapter.len() > 0 {
                    rows.push(format!("{}: {}", material_type.as_chapter(), self.chapter));
                }
                if self.page > 0 {
                    rows.push(format!("{}: {}/{}", material_type.as_page(), self.page, self.material_pages));
                }
                rows.push(format!("Material status: {}", self.material_status));
            }
            rows.push(format!("Added at (UTC): {}", added_at));

            if self.has_material_repeat() {
                let repeats_count = self.material_repeats_count.unwrap_or(0);
                let repeated_at = self.material_last_repeated_at
                    .unwrap().format("%Y-%m-%d");

                rows.push(format!("Repeats count: {}", repeats_count));
                rows.push(format!("Last repeated: {}, {}", repeated_at, self.repeated_ago()))
            }
            rows.push(format!("Total notes count: {}", self.notes_count));

            write!(f, "{}", rows.join("\n"))
        }
    }

    pub async fn get_note(pool: &PgPool) -> Result<RemindNote, sqlx::Error> {
        let stmt = sqlx::query!(r#"
        SELECT
            note_id AS "note_id!",
            material_title,
            material_authors,
            material_type AS "material_type?: MaterialTypes",
            content AS "content!",
            added_at AS "added_at!",
            chapter AS "chapter!",
            page AS "page!",
            tags AS "tags!",
            material_pages,
            total_notes_count AS "total_notes_count!",
            min_repeat_freq AS "min_repeat_freq!",
            material_status AS "material_status!",
            repeated_at,
            repeats_count AS "repeats_count?",
            COUNT(1) OVER () AS "notes_to_repeat!"
        FROM mvw_repeat_notes
        ORDER BY random()
        LIMIT 1
        "#)
            .fetch_one(pool)
            .await?;

        log::info!("Min repeat freq {}, total notes with it {}",
            stmt.min_repeat_freq, stmt.notes_to_repeat);

        Ok(RemindNote{
            note_id: stmt.note_id,
            content: stmt.content,
            page: stmt.page,
            chapter: stmt.chapter,
            added_at: stmt.added_at,
            tags: stmt.tags,
            notes_count: stmt.total_notes_count,
            material_title: stmt.material_title,
            material_authors: stmt.material_authors,
            material_type: stmt.material_type,
            material_pages: stmt.material_pages.unwrap_or(0),
            material_status: stmt.material_status,
            material_repeats_count: stmt.repeats_count,
            material_last_repeated_at: stmt.repeated_at,
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

    pub async fn refresh_repeat_notes_view(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!("REFRESH MATERIALIZED VIEW mvw_repeat_notes;").fetch_all(pool).await?;

        Ok(())
    }

    fn create_uuid() -> Uuid {
        Uuid::now_v7()
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

        #[derive(Debug)]
        pub struct Demark {
            content: String
        }

        impl Demark {
            pub fn new() -> Self {
                Demark { content: String::new() }
            }

            pub fn from<T: ToString>(content: T) -> Self {
                Demark { content: content.to_string() }
            }

            fn demark_bold(&mut self) -> &mut Self {
                let demark_bold_pattern = Regex::new(r#"\*{2}(.*?)\*{2}"#).unwrap();
                self.content = demark_bold_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("<b>{}</b>", &r[1])
                }).to_string();

                self
            }

            fn demark_italic(&mut self) -> &mut Self {
                // to allow unordered lists the first symbol should be not space
                let demark_italic_pattern = Regex::new(r#"\*(\S.*?)\*"#).unwrap();
                self.content = demark_italic_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("<i>{}</i>", &r[1])
                }).to_string();

                self
            }

            fn demark_code(&mut self) -> &mut Self {
                let demark_code_pattern = Regex::new(r#"`(.*?)`"#).unwrap();
                self.content = demark_code_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("<code>{}</code>", &r[1])
                }).to_string();

                self
            }

            fn demark_code_block(&mut self) -> &mut Self {
                let demark_code_pattern = Regex::new(r#"```\w*(.*?)```"#).unwrap();
                self.content = demark_code_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("<pre>{}</pre>", &r[1])
                }).to_string();

                self
            }

            fn demark_lt(&mut self) -> &mut Self {
                self.content = self.content.replace(" < ", " &lt; ");
                self
            }

            fn demark_gt(&mut self) -> &mut Self {
                self.content = self.content.replace(" > ", " &gt; ");
                self
            }

            fn demark_sup(&mut self) -> &mut Self {
                let demark_code_pattern = Regex::new(r#"<sup>(.*?)</sup>"#).unwrap();
                self.content = demark_code_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("^({})", &r[1])
                }).to_string();

                self
            }

            fn demark_sub(&mut self) -> &mut Self {
                let demark_code_pattern = Regex::new(r#"<sub>(.*?)</sub>"#).unwrap();
                self.content = demark_code_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("_({})", &r[1])
                }).to_string();

                self
            }

            fn demark_link(&mut self) -> &mut Self{
                let link_pattern = Regex::new(r#"\[([\w]*)\]\(([\w\d\.:\/-]*)\)"#).unwrap();

                self.content = link_pattern.replace_all(&self.content, |r: &Captures| {
                    format!("<a href='{}'>{}</a>", &r[2], &r[1])
                }).to_string();

                self
            }

            pub fn demark(mut self) -> String {
                self.demark_bold()
                    .demark_italic()
                    .demark_code_block()
                    .demark_code()
                    .demark_gt()
                    .demark_lt()
                    .demark_sub()
                    .demark_sup()
                    .demark_link();

                self.content
            }

        }


        pub fn demark(content: &str) -> String {
            demark_sub(&demark_sup(&demark_lt(&demark_gt(&demark_code(&demark_code_block(&demark_italic(&demark_bold(content)))))))).to_string()
        }

        fn demark_bold(content: &str) -> String {
            let demark_bold_pattern = Regex::new(r#"\*{2}(.*?)\*{2}"#).unwrap();
            demark_bold_pattern.replace_all(content, |r: &Captures| {
                format!("<b>{}</b>", &r[1])
            }).to_string()
        }

        fn demark_italic(content: &str) -> String {
            // to allow unordered lists the first symbol should be not space
            let demark_italic_pattern = Regex::new(r#"\*(\S.*?)\*"#).unwrap();
            demark_italic_pattern.replace_all(content, |r: &Captures| {
                format!("<i>{}</i>", &r[1])
            }).to_string()
        }

        fn demark_code(content: &str) -> String {
            let demark_code_pattern = Regex::new(r#"`(.*?)`"#).unwrap();
            demark_code_pattern.replace_all(content, |r: &Captures| {
                format!("<code>{}</code>", &r[1])
            }).to_string()
        }

        fn demark_code_block(content: &str) -> String {
            let demark_code_pattern = Regex::new(r#"```\w*(.*?)```"#).unwrap();
            demark_code_pattern.replace_all(content, |r: &Captures| {
                format!("<pre>{}</pre>", &r[1])
            }).to_string()
        }

        fn demark_lt(content: &str) -> String {
            content.replace(" < ", " &lt; ").to_string()
        }

        fn demark_gt(content: &str) -> String {
            content.replace(" > ", " &gt; ").to_string()
        }

        fn demark_sup(content: &str) -> String {
            let demark_code_pattern = Regex::new(r#"<sup>(.*?)</sup>"#).unwrap();
            demark_code_pattern.replace_all(content, |r: &Captures| {
                format!("^({})", &r[1])
            }).to_string()
        }

        fn demark_sub(content: &str) -> String {
            let demark_code_pattern = Regex::new(r#"<sub>(.*?)</sub>"#).unwrap();
            demark_code_pattern.replace_all(content, |r: &Captures| {
                format!("_({})", &r[1])
            }).to_string()
        }

        fn demark_link(content: &str) -> String {
            let link_pattern = Regex::new(r#"\[([\w]*)\]\(([\w\d\.:\/-]*)\)"#).unwrap();

            link_pattern.replace_all(content, |r: &Captures| {
                format!("<a href='{}'>{}</a>", &r[2], &r[1])
            }).to_string()
        }

        #[cfg(test)]
        mod test_demark {
            use crate::db::demark::Demark;

            #[test]
            fn test_demark_sup() {
                let mut res1 = Demark::from("some content <sup>some sup</sup> some end");
                let mut res2 = Demark::from("some content <sup>some123</sup> some end");

                assert_eq!(res1.demark_sup().content, "some content ^(some sup) some end");
                assert_eq!(res2.demark_sup().content, "some content ^(some123) some end");
            }

            #[test]
            fn test_demark_sub() {
                let mut res1 = Demark::from("some content <sub>some sub</sub> some end");
                let mut res2 = Demark::from("some content <sub>some123</sub> some end");

                assert_eq!(res1.demark_sub().content, "some content _(some sub) some end");
                assert_eq!(res2.demark_sub().content, "some content _(some123) some end");
            }

            #[test]
            fn test_demark_bold() {
                let mut res1 = Demark::from("some content **some bold** some end");
                let mut res2 = Demark::from("some content **some123** some end");
                let mut res3 = Demark::from("**some123**");
                let mut res4 = Demark::from("some content **some123**");
                let mut res5 = Demark::from("**some123** some end");

                assert_eq!(res1.demark_bold().content, "some content <b>some bold</b> some end");
                assert_eq!(res2.demark_bold().content, "some content <b>some123</b> some end");
                assert_eq!(res3.demark_bold().content, "<b>some123</b>");
                assert_eq!(res4.demark_bold().content, "some content <b>some123</b>");
                assert_eq!(res5.demark_bold().content, "<b>some123</b> some end");
            }

            #[test]
            fn test_demark_italic() {
                let mut res1 = Demark::from("some content *some italic* some end");
                let mut res2 = Demark::from("some content *some123* some end");
                let mut res3 = Demark::from("*some123*");
                let mut res4 = Demark::from("some content *some123*");
                let mut res5 = Demark::from("*some123* some end");
                let mut res6 = Demark::from("(*zero flag*)");
                let mut res7 = Demark::from("Комментарий @ava: *«kasdjfksj lasdj la asdklfjalsdk . asdfs: — asdjfks вы нам!»*.");
                
                assert_eq!(res1.demark_italic().content, "some content <i>some italic</i> some end");
                assert_eq!(res2.demark_italic().content, "some content <i>some123</i> some end");
                assert_eq!(res3.demark_italic().content, "<i>some123</i>");
                assert_eq!(res4.demark_italic().content, "some content <i>some123</i>");
                assert_eq!(res5.demark_italic().content, "<i>some123</i> some end");
                assert_eq!(res6.demark_italic().content, "(<i>zero flag</i>)");
                assert_eq!(res7.demark_italic().content, "Комментарий @ava: <i>«kasdjfksj lasdj la asdklfjalsdk . asdfs: — asdjfks вы нам!»</i>.");
            }

            #[test]
            fn test_demark_code_block() {
                let mut res = Demark::from("``` { 'type': 'http', 'asgi': {'version': '3.0', spec_version: '2.4'}, 'http_version': '1.1', 'server': ('127.0.0.1' 5000), 'client': ('127.0.0.1', 50422), 'scheme': 'http', 'method': 'GET', 'root_path': '', 'path': '/some/path/', 'raw_path': b'/some/path/', 'query_string': b'q=123', 'headers': [ (b'host', b'127.0.0.1:5000'), (b'user-agent', b'curl/7.81.0'), (b'accept', b'*/*') ] 'state': {} } ```");

                assert_eq!(res.demark_code_block().content, "<pre> { 'type': 'http', 'asgi': {'version': '3.0', spec_version: '2.4'}, 'http_version': '1.1', 'server': ('127.0.0.1' 5000), 'client': ('127.0.0.1', 50422), 'scheme': 'http', 'method': 'GET', 'root_path': '', 'path': '/some/path/', 'raw_path': b'/some/path/', 'query_string': b'q=123', 'headers': [ (b'host', b'127.0.0.1:5000'), (b'user-agent', b'curl/7.81.0'), (b'accept', b'*/*') ] 'state': {} } </pre>")
            }

            #[test]
            fn test_demark_code_block_with_language() {
                let mut res = Demark::from("```python async def application(scope, receive: Callable, send: Callable): ```");

                assert_eq!(res.demark_code_block().content, "<pre> async def application(scope, receive: Callable, send: Callable): </pre>")
            }

            #[test]
            fn test_demark_link() {
                let mut res = Demark::from("[ссылка](https://www.radware.com/blog/applicationdelivery/wpo/2014/11/real-cost-slow-time-vs-downtime-slides/), [ссылка](https://blog.research.google/2009/06/speed-matters.html)");

                assert_eq!(res.demark_link().content, "<a href='https://www.radware.com/blog/applicationdelivery/wpo/2014/11/real-cost-slow-time-vs-downtime-slides/'>ссылка</a>, <a href='https://blog.research.google/2009/06/speed-matters.html'>ссылка</a>")
            }

            #[test]
            fn test_demark() {
                let mut res = Demark::from("Комментарий @ava: *«kasdjfksj lasdj la asdklfjalsdk . asdfs: — asdjfks вы нам!»*.");
                let mut res2 = Demark::from("aksdflsk sdkfja:
* **sdfsdf** (*dfsdf*) — asda
* **adfsdj** (*dfsjdk*) — asdas!");

                assert_eq!(res.demark_italic().content, "Комментарий @ava: <i>«kasdjfksj lasdj la asdklfjalsdk . asdfs: — asdjfks вы нам!»</i>.");
                assert_eq!(res2.demark(), "aksdflsk sdkfja:\n* <b>sdfsdf</b> (<i>dfsdf</i>) — asda\n* <b>adfsdj</b> (<i>dfsjdk</i>) — asdas!");
            }
        }
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
