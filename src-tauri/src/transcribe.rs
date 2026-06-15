use base64::Engine as _;

pub async fn transcribe(
    wav_path: &std::path::Path,
    openai_key: &str,
    groq_key: &str,
    gemini_key: &str,
    provider: &str,
    language: &str,
) -> Result<String, String> {
    match provider {
        "gemini" => transcribe_gemini(wav_path, gemini_key, language).await,
        "groq" if !groq_key.is_empty() => {
            transcribe_whisper(
                wav_path,
                groq_key,
                "https://api.groq.com/openai/v1/audio/transcriptions",
                "whisper-large-v3-turbo",
                language,
            )
            .await
        }
        _ => {
            transcribe_whisper(
                wav_path,
                openai_key,
                "https://api.openai.com/v1/audio/transcriptions",
                "whisper-1",
                language,
            )
            .await
        }
    }
}

async fn transcribe_whisper(
    wav_path: &std::path::Path,
    key: &str,
    url: &str,
    model: &str,
    language: &str,
) -> Result<String, String> {
    let wav_bytes = std::fs::read(wav_path).map_err(|e| e.to_string())?;

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

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

async fn transcribe_gemini(
    wav_path: &std::path::Path,
    gemini_key: &str,
    language: &str,
) -> Result<String, String> {
    let wav_bytes = std::fs::read(wav_path).map_err(|e| e.to_string())?;
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

    let lang_hint = if !language.is_empty() && language != "auto" {
        format!(" The spoken language is {language}.")
    } else {
        String::new()
    };

    let prompt = format!(
        "Transcribe this audio exactly as spoken. Output only the transcription — \
         no commentary, no timestamps, no speaker labels.{lang_hint}"
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [
                {"inline_data": {"mime_type": "audio/wav", "data": audio_b64}},
                {"text": prompt}
            ]
        }]
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={gemini_key}"
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini transcription API {status}: {text}"));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| format!("Unexpected Gemini response: {json}"))
        .map(|s| s.trim().to_string())
}
