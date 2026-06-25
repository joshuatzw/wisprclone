use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// High-frequency English words excluded from vocabulary tracking.
// We only want to learn proper nouns, slang, technical terms, etc.
static COMMON_WORDS: &[&str] = &[
    "the", "be", "to", "of", "and", "a", "in", "that", "have", "it",
    "for", "not", "on", "with", "he", "as", "you", "do", "at", "this",
    "but", "his", "by", "from", "they", "we", "say", "her", "she", "or",
    "an", "will", "my", "one", "all", "would", "there", "their", "what",
    "so", "up", "out", "if", "about", "who", "get", "which", "go", "me",
    "when", "make", "can", "like", "time", "no", "just", "him", "know",
    "take", "people", "into", "year", "your", "good", "some", "could",
    "them", "see", "other", "than", "then", "now", "look", "only", "come",
    "its", "over", "think", "also", "back", "after", "use", "two", "how",
    "our", "work", "first", "well", "way", "even", "new", "want", "because",
    "any", "these", "give", "day", "most", "us", "great", "need", "large",
    "often", "hand", "high", "place", "hold", "turn", "here", "why", "help",
    "put", "different", "away", "again", "off", "should", "through", "going",
    "where", "much", "too", "very", "got", "yes", "was", "had", "been",
    "has", "are", "were", "did", "does", "don", "isn", "aren", "wasn",
    "didn", "hasn", "haven", "couldn", "wouldn", "shouldn", "won",
    "okay", "yeah", "hey", "hi", "um", "uh", "er", "actually", "basically",
    "literally", "really", "right", "kind", "sort", "thing", "things",
    "stuff", "bit", "lot", "something", "anything", "everything", "nothing",
    "someone", "anyone", "everyone", "maybe", "probably", "might", "still",
    "already", "always", "never", "sometimes", "usually", "though",
    "although", "however", "therefore", "since", "while", "before", "after",
    "during", "without", "within", "between", "among", "around", "against",
    "along", "beside", "besides", "beyond", "down", "next", "last",
    "little", "small", "big", "same", "old", "long", "short", "own",
    "each", "both", "either", "neither", "every", "another", "such",
    "more", "most", "less", "least", "few", "many", "several", "quite",
    "rather", "enough", "whatever", "whenever", "wherever", "whoever",
    "am", "is", "are", "was", "were", "being", "am",
    "have", "has", "had", "will", "shall", "may", "must",
    "get", "got", "getting", "gone", "went", "came", "knew", "known",
    "saw", "seen", "said", "wanted", "used", "found", "gave", "given",
    "told", "worked", "called", "tried", "asked", "needed", "felt",
    "became", "left", "kept", "began", "seemed", "helped", "showed",
    "heard", "played", "moved", "lived", "believed", "held", "brought",
    "happened", "wrote", "sat", "stood", "lost", "paid", "met", "led",
    "understood", "watched", "followed", "stopped", "created", "spoke",
    "spent", "grew", "opened", "walked", "offered", "remembered", "loved",
    "considered", "appeared", "bought", "waited", "served", "died", "sent",
    "expected", "built", "stayed", "fell", "reached", "killed", "remained",
    "suggested", "raised", "passed", "sold", "required", "reported",
    "decided", "pulled",
    // contractions (often appear as-is after cleanup)
    "i'm", "i've", "i'll", "i'd", "it's", "he's", "she's", "we're",
    "they're", "you're", "that's", "there's", "what's", "who's",
    "don't", "doesn't", "didn't", "won't", "wouldn't", "couldn't",
    "shouldn't", "hasn't", "haven't", "isn't", "aren't", "wasn't",
    "weren't", "can't", "let's",
];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VocabStore {
    pub words: HashMap<String, u32>,
}

impl VocabStore {
    /// Extract words from a transcript and increment their usage counts.
    pub fn learn(&mut self, text: &str) {
        let common: std::collections::HashSet<&str> = COMMON_WORDS.iter().copied().collect();
        for token in text.split_whitespace() {
            // Strip surrounding punctuation, keep hyphens and apostrophes
            let clean: String = token
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '-')
                .collect();
            // Skip very short tokens and common words
            if clean.len() < 4 {
                continue;
            }
            if common.contains(clean.to_lowercase().as_str()) {
                continue;
            }
            // Preserve original casing (proper nouns keep their capital)
            *self.words.entry(clean).or_insert(0) += 1;
        }
    }

    /// Manually add a word, immediately marking it as active (count >= 2).
    pub fn add(&mut self, word: String) {
        let entry = self.words.entry(word).or_insert(0);
        if *entry < 2 {
            *entry = 2;
        }
    }

    pub fn remove(&mut self, word: &str) {
        self.words.remove(word);
    }

    /// Words with count >= 2, sorted by frequency, capped at 50 for API prompts.
    pub fn active_words(&self) -> Vec<String> {
        let mut entries: Vec<(&String, &u32)> = self.words.iter()
            .filter(|(_, &c)| c >= 2)
            .collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        entries.into_iter().take(50).map(|(w, _)| w.clone()).collect()
    }

    /// All words sorted by frequency descending, for the settings UI.
    pub fn all_sorted(&self) -> Vec<(String, u32)> {
        let mut entries: Vec<(String, u32)> = self.words.iter()
            .map(|(w, &c)| (w.clone(), c))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        entries
    }
}

pub fn load(dir: &Path) -> VocabStore {
    let path = dir.join("vocabulary.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, store: &VocabStore) {
    std::fs::create_dir_all(dir).ok();
    let path = dir.join("vocabulary.json");
    if let Ok(json) = serde_json::to_string_pretty(store) {
        std::fs::write(path, json).ok();
    }
}
