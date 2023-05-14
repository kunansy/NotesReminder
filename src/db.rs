pub mod db {
    use std::collections::HashMap;
    use sqlx::postgres::PgPoolOptions;

    struct RemindInfo(i32, chrono::NaiveDateTime);
    pub struct Note { }

    pub async fn get_note(pool: &PgPoolOptions) -> Note {
        Note
    }

    pub async fn insert_note_history(pool: &PgPoolOptions,
                                     note_id: &String,
                                     user_id: i32) {

    }

    async fn get_notes_count(pool: &PgPoolOptions) -> i32 {
        42
    }

    async fn get_material_repeat_info(pool: &PgPoolOptions,
                                      material_id: &String) -> RemindInfo {
        RemindInfo(1, chrono::NaiveDateTime)
    }

    async fn get_remind_statistics(pool: &PgPoolOptions) -> HashMap<String, i32> {
        HashMap::new()
    }

    fn get_remind_note_id(stats: HashMap<String, i32>) -> String {
        "".to_string()
    }
}