use serde_json::{json, Value};

const SYSTEM: &str = "\
You are a dictation cleanup assistant. Convert raw speech transcription into clean, finished writing.

Rules:
- Handle spoken punctuation: \"comma\" → ,  \"period\"/\"full stop\" → .  \"question mark\" → ?  \
\"exclamation mark\" → !  \"colon\" → :  \"semicolon\" → ;  \
\"new line\" → line break  \"new paragraph\" → blank line
- Remove filler words (um, uh, er, like, you know) when they are clearly verbal fillers
- Fix obvious grammar and capitalisation errors; add sentence-ending punctuation where missing
- Preserve the speaker's voice and exact meaning — do not rephrase, expand, or summarise
- Output only the cleaned text with no commentary or preamble
- If the input is empty or completely unintelligible, output nothing";

pub async fn cleanup_transcript(raw: &str, api_key: &str) -> Result<String, String> {
    if raw.trim().is_empty() {
        return Ok(String::new());
    }

    let body = json!({
        "model": "claude-haiku-4-5",
        "max_tokens": 1024,
        "system": SYSTEM,
        "messages": [{"role": "user", "content": raw}]
    });

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Claude API {status}: {text}"));
    }

    let json: Value = response.json().await.map_err(|e| e.to_string())?;
    json["content"][0]["text"]
        .as_str()
        .ok_or_else(|| format!("Unexpected response: {json}"))
        .map(|s| s.trim().to_string())
}
