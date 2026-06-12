pub async fn send_to_whisper(wav_path: &std::path::Path, api_key: &str) -> Result<String, String> {
    let wav_bytes = std::fs::read(wav_path).map_err(|e| e.to_string())?;

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-1")
        .text("language", "en");

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Whisper API {status}: {body}"));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    json["text"]
        .as_str()
        .ok_or_else(|| format!("Unexpected response: {json}"))
        .map(|s| s.trim().to_string())
}
