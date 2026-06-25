use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

mod audio;
mod cleanup;
mod config;
mod context;
mod corrections;
mod history;
mod hotkey;
mod inject;
mod normalize;
mod transcribe;
mod vocabulary;
mod volume;

use audio::AudioRecorder;

struct AppState {
    recorder: Mutex<AudioRecorder>,
    openai_key: Mutex<String>,
    anthropic_key: Mutex<String>,
    groq_key: Mutex<String>,
    gemini_key: Mutex<String>,
    config: Mutex<config::AppConfig>,
    history: Mutex<Vec<history::HistoryEntry>>,
    vocabulary: Mutex<vocabulary::VocabStore>,
    corrections: Mutex<corrections::CorrectionsStore>,
    app_data_dir: PathBuf,
    pending_context: Mutex<context::AppContext>,
}

#[tauri::command]
fn set_api_key(state: tauri::State<AppState>, name: String, key: String) {
    let mut cfg = state.config.lock().unwrap();
    match name.as_str() {
        "openai" => {
            *state.openai_key.lock().unwrap() = key.clone();
            cfg.openai_key = key;
        }
        "anthropic" => {
            *state.anthropic_key.lock().unwrap() = key.clone();
            cfg.anthropic_key = key;
        }
        "groq" => {
            *state.groq_key.lock().unwrap() = key.clone();
            cfg.groq_key = key;
        }
        "gemini" => {
            *state.gemini_key.lock().unwrap() = key.clone();
            cfg.gemini_key = key;
        }
        _ => return,
    }
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_cleanup_enabled(state: tauri::State<AppState>, enabled: bool) {
    let mut cfg = state.config.lock().unwrap();
    cfg.cleanup_enabled = enabled;
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_stt_provider(state: tauri::State<AppState>, provider: String) {
    let mut cfg = state.config.lock().unwrap();
    cfg.stt_provider = match provider.as_str() {
        "groq" => config::SttProvider::Groq,
        "gemini" => config::SttProvider::Gemini,
        _ => config::SttProvider::Openai,
    };
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_cleanup_provider(state: tauri::State<AppState>, provider: String) {
    let mut cfg = state.config.lock().unwrap();
    cfg.cleanup_provider = match provider.as_str() {
        "gemini" => config::CleanupProvider::Gemini,
        _ => config::CleanupProvider::Anthropic,
    };
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_language(state: tauri::State<AppState>, language: String) {
    let mut cfg = state.config.lock().unwrap();
    cfg.language = language;
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<audio::AudioDeviceInfo>, String> {
    audio::list_input_devices()
}

#[tauri::command]
fn set_input_device(state: tauri::State<AppState>, device: String) {
    let mut cfg = state.config.lock().unwrap();
    cfg.input_device = device;
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_context_awareness_enabled(state: tauri::State<AppState>, enabled: bool) {
    let mut cfg = state.config.lock().unwrap();
    cfg.context_awareness_enabled = enabled;
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_tone_style(state: tauri::State<AppState>, tone: String) {
    let mut cfg = state.config.lock().unwrap();
    cfg.tone_style = match tone.as_str() {
        "casual" => config::ToneStyle::Casual,
        _ => config::ToneStyle::Formal,
    };
    config::save(&state.app_data_dir, &cfg);
}

#[tauri::command]
fn set_hotkey_combo(state: tauri::State<AppState>, hotkey: String) {
    hotkey::reset();
    let combo = match hotkey.as_str() {
        "right_alt" => config::HotkeyCombo::RightAlt,
        "ctrl_shift" => config::HotkeyCombo::CtrlShift,
        "ctrl_alt" => config::HotkeyCombo::CtrlAlt,
        _ => config::HotkeyCombo::CtrlWin,
    };
    hotkey::set_combo(combo.to_u8());
    let mut cfg = state.config.lock().unwrap();
    cfg.hotkey = combo;
    config::save(&state.app_data_dir, &cfg);
}

#[derive(serde::Serialize)]
struct Settings {
    cleanup_enabled: bool,
    stt_provider: String,
    cleanup_provider: String,
    language: String,
    hotkey: String,
    context_awareness_enabled: bool,
    tone_style: String,
    input_device: String,
    openai_key: String,
    anthropic_key: String,
    groq_key: String,
    gemini_key: String,
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    let cfg = state.config.lock().unwrap();
    Settings {
        cleanup_enabled: cfg.cleanup_enabled,
        stt_provider: cfg.stt_provider.as_str().to_string(),
        cleanup_provider: cfg.cleanup_provider.as_str().to_string(),
        language: cfg.language.clone(),
        hotkey: cfg.hotkey.as_str().to_string(),
        context_awareness_enabled: cfg.context_awareness_enabled,
        tone_style: cfg.tone_style.as_str().to_string(),
        input_device: cfg.input_device.clone(),
        openai_key: cfg.openai_key.clone(),
        anthropic_key: cfg.anthropic_key.clone(),
        groq_key: cfg.groq_key.clone(),
        gemini_key: cfg.gemini_key.clone(),
    }
}

#[tauri::command]
fn get_history(state: tauri::State<AppState>) -> Vec<history::HistoryEntry> {
    state.history.lock().unwrap().clone()
}

#[tauri::command]
fn delete_history_entry(state: tauri::State<AppState>, id: u64) {
    let mut entries = state.history.lock().unwrap();
    entries.retain(|e| e.id != id);
    history::save(&state.app_data_dir, &entries);
}

#[tauri::command]
fn update_history_text(state: tauri::State<AppState>, id: u64, new_text: String) {
    let old_text = {
        let mut entries = state.history.lock().unwrap();
        let Some(entry) = entries.iter_mut().find(|e| e.id == id) else { return };
        let old = entry.text.clone();
        entry.text = new_text.clone();
        history::save(&state.app_data_dir, &entries);
        old
    };

    // Learn new correction rules from the edit
    let correction_targets: Vec<String> = {
        let mut corr = state.corrections.lock().unwrap();
        corr.learn_from_diff(&old_text, &new_text);
        corrections::save(&state.app_data_dir, &corr);
        corr.rules.values().cloned().collect()
    };

    // Promote corrected forms in vocabulary so they're used as active STT hints
    {
        let mut vocab = state.vocabulary.lock().unwrap();
        vocab.learn(&new_text);
        for correct_form in correction_targets {
            vocab.add(correct_form);
        }
        vocabulary::save(&state.app_data_dir, &vocab);
    }
}

#[derive(serde::Serialize)]
struct VocabWord {
    word: String,
    count: u32,
}

#[tauri::command]
fn get_vocabulary(state: tauri::State<AppState>) -> Vec<VocabWord> {
    state
        .vocabulary
        .lock()
        .unwrap()
        .all_sorted()
        .into_iter()
        .map(|(word, count)| VocabWord { word, count })
        .collect()
}

#[tauri::command]
fn add_vocab_word(state: tauri::State<AppState>, word: String) {
    let mut vocab = state.vocabulary.lock().unwrap();
    vocab.add(word);
    vocabulary::save(&state.app_data_dir, &vocab);
}

#[tauri::command]
fn delete_vocab_word(state: tauri::State<AppState>, word: String) {
    let mut vocab = state.vocabulary.lock().unwrap();
    vocab.remove(&word);
    vocabulary::save(&state.app_data_dir, &vocab);
}

struct RecordingJob {
    openai_key: String,
    anthropic_key: String,
    groq_key: String,
    gemini_key: String,
    cleanup_enabled: bool,
    stt_provider: String,
    cleanup_provider: String,
    language: String,
    app_context: context::AppContext,
    tone_style: config::ToneStyle,
    vocab_words: Vec<String>,
    corrections: corrections::CorrectionsStore,
}

fn apply_tone_fallback(text: &str, tone: &config::ToneStyle) -> String {
    match tone {
        config::ToneStyle::Formal => {
            let mut chars = text.chars();
            let capitalized = match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
                None => return text.to_string(),
            };
            if capitalized.ends_with(|c: char| c.is_alphanumeric()) {
                format!("{capitalized}.")
            } else {
                capitalized
            }
        }
        config::ToneStyle::Casual => text.trim_end_matches('.').to_string(),
    }
}

async fn process_recording(handle: AppHandle, job: RecordingJob) {
    let wav_path = std::env::temp_dir().join("wispr_recording.wav");

    normalize::boost_quiet(&wav_path);
    handle.emit("recording-state", "transcribing").ok();

    let raw = match transcribe::transcribe(
        &wav_path,
        &job.openai_key,
        &job.groq_key,
        &job.gemini_key,
        &job.stt_provider,
        &job.language,
        &job.vocab_words,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[wispr] Transcription error: {e}");
            handle.emit("recording-state", "idle").ok();
            handle.emit("error-message", e).ok();
            return;
        }
    };
    println!("[wispr] Raw: {raw}");
    let raw = job.corrections.apply(&raw);

    let cleanup_key_ok = match job.cleanup_provider.as_str() {
        "gemini" => !job.gemini_key.is_empty(),
        _ => !job.anthropic_key.is_empty(),
    };

    let final_text = if job.cleanup_enabled && cleanup_key_ok {
        handle.emit("recording-state", "cleaning").ok();
        match cleanup::cleanup_transcript(
            &raw,
            &job.anthropic_key,
            &job.gemini_key,
            &job.cleanup_provider,
            &job.app_context,
            &job.tone_style,
            &job.vocab_words,
        )
        .await
        {
            Ok(cleaned) if !cleaned.is_empty() => {
                println!("[wispr] Cleaned: {cleaned}");
                cleaned
            }
            Ok(_) => raw,
            Err(e) => {
                eprintln!("[wispr] Cleanup error: {e}");
                apply_tone_fallback(&raw, &job.tone_style)
            }
        }
    } else {
        apply_tone_fallback(&raw, &job.tone_style)
    };

    if let Err(e) = inject::paste_text(&final_text) {
        eprintln!("[wispr] Inject error: {e}");
    }

    let state = handle.state::<AppState>();
    let entry = history::push(
        &mut state.history.lock().unwrap(),
        final_text.clone(),
        &state.app_data_dir,
    );
    {
        let mut vocab = state.vocabulary.lock().unwrap();
        vocab.learn(&final_text);
        vocabulary::save(&state.app_data_dir, &vocab);
    }
    handle.emit("history-entry", entry).ok();
    handle.emit("recording-state", "idle").ok();
    handle.emit("transcript", final_text).ok();
}

#[cfg(windows)]
fn show_already_running_dialog(hotkey: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONINFORMATION, MB_OK,
    };
    let title: Vec<u16> = "WisprClone\0".encode_utf16().collect();
    let msg = format!(
        "WisprClone is already running.\r\nPress {} to start recording!\0",
        hotkey
    );
    let msg_w: Vec<u16> = msg.encode_utf16().collect();
    unsafe {
        MessageBoxW(0, msg_w.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Single-instance guard: bind a local port exclusively; a second launch will fail to bind
    // and see the "already running" dialog instead. The listener is held for the process lifetime.
    let _instance_guard = match std::net::TcpListener::bind("127.0.0.1:37129") {
        Ok(listener) => listener,
        Err(_) => {
            #[cfg(windows)]
            {
                let hotkey_label = std::env::var("APPDATA")
                    .ok()
                    .map(|appdata| {
                        let dir =
                            std::path::PathBuf::from(appdata).join("com.joshuatan.wispr-clone");
                        let cfg = config::load(&dir);
                        match cfg.hotkey {
                            config::HotkeyCombo::CtrlWin => "Ctrl+Win",
                            config::HotkeyCombo::RightAlt => "Right Alt",
                            config::HotkeyCombo::CtrlShift => "Ctrl+Shift",
                            config::HotkeyCombo::CtrlAlt => "Ctrl+Alt",
                        }
                    })
                    .unwrap_or("Ctrl+Win");
                show_already_running_dialog(hotkey_label);
            }
            std::process::exit(0);
        }
    };

    let recorder = AudioRecorder::new().expect("Failed to initialise audio recorder");
    let (hotkey_tx, hotkey_rx) = mpsc::sync_channel::<bool>(4);

    hotkey::start(hotkey_tx);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            set_api_key,
            set_cleanup_enabled,
            set_stt_provider,
            set_cleanup_provider,
            set_language,
            list_audio_devices,
            set_input_device,
            set_hotkey_combo,
            set_context_awareness_enabled,
            set_tone_style,
            get_settings,
            get_history,
            delete_history_entry,
            update_history_text,
            get_vocabulary,
            add_vocab_word,
            delete_vocab_word,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    window.hide().ok();
                }
            }
        })
        .setup(move |app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            let history_entries = history::load(&app_data_dir);
            let app_config = config::load(&app_data_dir);
            let vocab_store = vocabulary::load(&app_data_dir);
            let corrections_store = corrections::load(&app_data_dir);

            hotkey::set_combo(app_config.hotkey.to_u8());

            app.manage(AppState {
                recorder: Mutex::new(recorder),
                openai_key: Mutex::new(app_config.openai_key.clone()),
                anthropic_key: Mutex::new(app_config.anthropic_key.clone()),
                groq_key: Mutex::new(app_config.groq_key.clone()),
                gemini_key: Mutex::new(app_config.gemini_key.clone()),
                config: Mutex::new(app_config),
                history: Mutex::new(history_entries),
                vocabulary: Mutex::new(vocab_store),
                corrections: Mutex::new(corrections_store),
                app_data_dir,
                pending_context: Mutex::new(context::AppContext::General),
            });

            let settings_item =
                MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item =
                MenuItem::with_id(app, "quit", "Quit WisprClone", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &sep, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("no default icon").clone())
                .menu(&menu)
                .tooltip("WisprClone — Hold hotkey to record")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        if let Some(w) = app.get_webview_window("main") {
                            w.show().ok();
                            w.set_focus().ok();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                w.hide().ok();
                            } else {
                                w.show().ok();
                                w.set_focus().ok();
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(overlay) = app.get_webview_window("overlay") {
                if let Ok(Some(monitor)) = overlay.primary_monitor() {
                    let phys = monitor.size();
                    let scale = monitor.scale_factor();
                    let lw = phys.width as f64 / scale;
                    let lh = phys.height as f64 / scale;
                    overlay
                        .set_position(tauri::LogicalPosition::new(
                            (lw - 64.0) / 2.0,
                            lh - 44.0 - 56.0,
                        ))
                        .ok();
                }
                overlay.show().ok();
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for is_pressed in hotkey_rx {
                    let state = handle.state::<AppState>();

                    if is_pressed {
                        let mut rec = state.recorder.lock().unwrap();
                        if !rec.is_recording() {
                            *state.pending_context.lock().unwrap() =
                                context::detect_focused_app();
                            let input_device = state.config.lock().unwrap().input_device.clone();
                            match rec.start(&input_device) {
                                Ok(()) => {
                                    volume::duck();
                                    println!("[wispr] Recording started");
                                    handle.emit("recording-state", "recording").ok();
                                }
                                Err(e) => {
                                    eprintln!("[wispr] Start error: {e}");
                                    handle.emit("recording-state", "idle").ok();
                                    handle.emit("error-message", e).ok();
                                }
                            }
                        }
                        continue;
                    }

                    // Key released — restore audio immediately, then process
                    volume::unduck();
                    let wav_path = std::env::temp_dir().join("wispr_recording.wav");
                    let capture = match state.recorder.lock().unwrap().stop_and_save(&wav_path) {
                        Ok(info) => info,
                        Err(e) => {
                            eprintln!("[wispr] Save error: {e}");
                            handle.emit("recording-state", "idle").ok();
                            continue;
                        }
                    };
                    println!(
                        "[wispr] Capture: device=\"{}\" duration={:.2}s rate={}Hz channels={} peak={:.4} rms={:.4}",
                        capture.device_name,
                        capture.duration_secs,
                        capture.sample_rate,
                        capture.input_channels,
                        capture.peak,
                        capture.rms
                    );
                    handle.emit("audio-capture", capture).ok();

                    let stt_provider = state.config.lock().unwrap().stt_provider.as_str().to_string();
                    let openai_key = state.openai_key.lock().unwrap().clone();
                    let groq_key = state.groq_key.lock().unwrap().clone();
                    let gemini_key = state.gemini_key.lock().unwrap().clone();

                    let stt_key_ok = match stt_provider.as_str() {
                        "gemini" => !gemini_key.is_empty(),
                        "groq"   => !groq_key.is_empty(),
                        _        => !openai_key.is_empty(),
                    };
                    if !stt_key_ok {
                        let provider = match stt_provider.as_str() {
                            "gemini" => "Gemini",
                            "groq"   => "Groq",
                            _        => "OpenAI",
                        };
                        handle.emit("recording-state", "idle").ok();
                        handle.emit("error-message", format!("{provider} API key not set")).ok();
                        continue;
                    }

                    let (cleanup_enabled, cleanup_provider, language, context_enabled, tone_style) = {
                        let cfg = state.config.lock().unwrap();
                        (
                            cfg.cleanup_enabled,
                            cfg.cleanup_provider.as_str().to_string(),
                            cfg.language.clone(),
                            cfg.context_awareness_enabled,
                            cfg.tone_style.clone(),
                        )
                    };
                    let app_context = if context_enabled {
                        state.pending_context.lock().unwrap().clone()
                    } else {
                        context::AppContext::General
                    };
                    let vocab_words = state.vocabulary.lock().unwrap().active_words();
                    let corrections = state.corrections.lock().unwrap().clone();
                    println!("[wispr] Context: {}", app_context.as_str());
                    println!("[wispr] Vocab hints: {} words", vocab_words.len());
                    println!("[wispr] Corrections: {} rules", corrections.rules.len());

                    tauri::async_runtime::spawn(process_recording(
                        handle.clone(),
                        RecordingJob {
                            openai_key,
                            anthropic_key: state.anthropic_key.lock().unwrap().clone(),
                            groq_key,
                            gemini_key,
                            cleanup_enabled,
                            stt_provider,
                            cleanup_provider,
                            language,
                            app_context,
                            tone_style,
                            vocab_words,
                            corrections,
                        },
                    ));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
