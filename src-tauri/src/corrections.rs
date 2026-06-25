use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CorrectionsStore {
    // lowercased wrong form -> preferred correct form
    pub rules: HashMap<String, String>,
}

/// Extract the alphanumeric core of a token, stripping surrounding punctuation.
fn core_of(token: &str) -> &str {
    let start = match token.find(|c: char| c.is_alphanumeric()) {
        Some(i) => i,
        None => return "",
    };
    let end = token
        .rfind(|c: char| c.is_alphanumeric())
        .map(|i| i + token[i..].chars().next().map_or(0, |c| c.len_utf8()))
        .unwrap_or(0);
    if start < end { &token[start..end] } else { "" }
}

impl CorrectionsStore {
    /// Replace any word whose core matches a known rule, preserving surrounding punctuation.
    pub fn apply(&self, text: &str) -> String {
        if self.rules.is_empty() {
            return text.to_string();
        }
        let mut result = String::with_capacity(text.len());
        let mut first = true;
        for token in text.split_whitespace() {
            if !first { result.push(' '); }
            first = false;
            let core = core_of(token);
            if core.is_empty() {
                result.push_str(token);
                continue;
            }
            let key = core.to_lowercase();
            if let Some(correct) = self.rules.get(&key) {
                // find the core's byte position in the original token
                if let Some(start) = token.find(core) {
                    let end = start + core.len();
                    result.push_str(&token[..start]);
                    result.push_str(correct);
                    result.push_str(&token[end..]);
                } else {
                    result.push_str(token);
                }
            } else {
                result.push_str(token);
            }
        }
        result
    }

    /// Compare two texts word-by-word (same word count only) and register changed words as rules.
    pub fn learn_from_diff(&mut self, old_text: &str, new_text: &str) {
        let old_words: Vec<&str> = old_text.split_whitespace().collect();
        let new_words: Vec<&str> = new_text.split_whitespace().collect();
        if old_words.len() != new_words.len() {
            return;
        }
        for (old_token, new_token) in old_words.iter().zip(new_words.iter()) {
            let old_core = core_of(old_token);
            let new_core = core_of(new_token);
            if old_core.is_empty() || new_core.is_empty() {
                continue;
            }
            let old_lower = old_core.to_lowercase();
            let new_lower = new_core.to_lowercase();
            if old_lower != new_lower {
                self.rules.insert(old_lower, new_core.to_string());
            }
        }
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, wrong: &str) {
        self.rules.remove(&wrong.to_lowercase());
    }
}

pub fn load(dir: &Path) -> CorrectionsStore {
    let path = dir.join("corrections.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, store: &CorrectionsStore) {
    std::fs::create_dir_all(dir).ok();
    let path = dir.join("corrections.json");
    if let Ok(json) = serde_json::to_string_pretty(store) {
        std::fs::write(path, json).ok();
    }
}
