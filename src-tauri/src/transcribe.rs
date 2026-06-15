pub async fn transcribe(
    wav_path: &std::path::Path,
    openai_key: &str,
    groq_key: &str,
    provider: &str,
    language: &str,
) -> Result<String, String> {
    let (url, key, model) = if provider == "groq" && !groq_key.is_empty() {
        (
            "https://api.groq.com/openai/v1/audio/transcriptions",
            groq_key,
            "whisper-large-v3-turbo",
        )
    } else {
        (
            "https://api.openai.com/v1/audio/transcriptions",
            openai_key,
            "whisper-1",
        )
    };

    let wav_bytes = std::fs::read(wav_path).map_err(|e| e.to_string())?;

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

    // Empty string or "auto" means let Whisper detect the language automatically
    if !language.is_empty() && language != "auto" {
        form = form.text("language", language.to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Transcription API {status}: {body}"));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    json["text"]
        .as_str()
        .ok_or_else(|| format!("Unexpected response: {json}"))
        .map(|s| s.trim().to_string())
}
