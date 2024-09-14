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
    pub priority_months: f32
}

pub async fn get_repeat_queue(tracker_url: &str) -> Result<Vec<RepeatItem>, String> {
    let url = format!("{}/materials/repeat-queue", tracker_url).parse()
        .map_err(|e: uri::InvalidUri| e.to_string())?;

    log::debug!("Getting repeat queue from {}", url);
    let client = Client::new();

    let resp = client.get(url)
        .await.map_err(|e| format!("GET fails: {}", e.to_string()))?;
    let body = hyper::body::aggregate(resp)
        .await.map_err(|e| e.to_string())?;

    let json: Vec<RepeatItem> = serde_json::from_reader(body.reader())
        .map_err(|e| format!("Could not parse json: {}", e.to_string()))?;

    log::debug!("{} queue items found", &json.len());
    Ok(json)
}
