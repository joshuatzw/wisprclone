use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

mod audio;
mod cleanup;
mod history;
mod hotkey;
mod inject;
mod transcribe;

use audio::AudioRecorder;

struct AppState {
    recorder: Mutex<AudioRecorder>,
    openai_key: Mutex<String>,
    anthropic_key: Mutex<String>,
    history: Mutex<Vec<history::HistoryEntry>>,
    app_data_dir: PathBuf,
}

#[tauri::command]
fn set_openai_key(state: tauri::State<AppState>, key: String) {
    *state.openai_key.lock().unwrap() = key;
}

#[tauri::command]
fn set_anthropic_key(state: tauri::State<AppState>, key: String) {
    *state.anthropic_key.lock().unwrap() = key;
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

fn set_overlay_visible(app: &AppHandle, visible: bool) {
    if let Some(w) = app.get_webview_window("overlay") {
        if visible {
            w.show().ok();
        } else {
            w.hide().ok();
        }
    }
}

async fn process_recording(handle: AppHandle, openai_key: String, anthropic_key: String) {
    let wav_path = std::env::temp_dir().join("wispr_recording.wav");

    handle.emit("recording-state", "transcribing").ok();
    let raw = match transcribe::send_to_whisper(&wav_path, &openai_key).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[wispr] Whisper error: {e}");
            handle.emit("recording-state", "idle").ok();
            handle.emit("error-message", e).ok();
            set_overlay_visible(&handle, false);
            return;
        }
    };
    println!("[wispr] Raw: {raw}");

    let final_text = if anthropic_key.is_empty() {
        raw
    } else {
        handle.emit("recording-state", "cleaning").ok();
        match cleanup::cleanup_transcript(&raw, &anthropic_key).await {
            Ok(cleaned) if !cleaned.is_empty() => {
                println!("[wispr] Cleaned: {cleaned}");
                cleaned
            }
            Ok(_) => raw,
            Err(e) => {
                eprintln!("[wispr] Cleanup error: {e}");
                raw
            }
        }
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
    handle.emit("history-entry", entry).ok();

    handle.emit("recording-state", "idle").ok();
    handle.emit("transcript", final_text).ok();
    set_overlay_visible(&handle, false);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recorder = AudioRecorder::new().expect("Failed to initialise audio recorder");
    let (hotkey_tx, hotkey_rx) = mpsc::sync_channel::<bool>(4);

    hotkey::start(hotkey_tx);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            set_openai_key,
            set_anthropic_key,
            get_history,
            delete_history_entry,
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

            app.manage(AppState {
                recorder: Mutex::new(recorder),
                openai_key: Mutex::new(String::new()),
                anthropic_key: Mutex::new(String::new()),
                history: Mutex::new(history_entries),
                app_data_dir,
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
                .tooltip("WisprClone — Hold Ctrl+Win to record")
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

            // Position overlay at center-bottom of primary screen, above the taskbar
            if let Some(overlay) = app.get_webview_window("overlay") {
                if let Ok(Some(monitor)) = overlay.primary_monitor() {
                    let phys = monitor.size();
                    let scale = monitor.scale_factor();
                    let lw = phys.width as f64 / scale;
                    let lh = phys.height as f64 / scale;
                    overlay
                        .set_position(tauri::LogicalPosition::new(
                            (lw - 44.0) / 2.0,
                            lh - 44.0 - 56.0,
                        ))
                        .ok();
                }
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for is_pressed in hotkey_rx {
                    let state = handle.state::<AppState>();

                    if is_pressed {
                        let mut rec = state.recorder.lock().unwrap();
                        if !rec.is_recording() {
                            match rec.start() {
                                Ok(()) => {
                                    println!("[wispr] Recording started");
                                    handle.emit("recording-state", "recording").ok();
                                    set_overlay_visible(&handle, true);
                                }
                                Err(e) => eprintln!("[wispr] Start error: {e}"),
                            }
                        }
                        continue;
                    }

                    // Key released — stop and process
                    let wav_path = std::env::temp_dir().join("wispr_recording.wav");
                    if let Err(e) = state.recorder.lock().unwrap().stop_and_save(&wav_path) {
                        eprintln!("[wispr] Save error: {e}");
                        handle.emit("recording-state", "idle").ok();
                        set_overlay_visible(&handle, false);
                        continue;
                    }

                    let openai_key = state.openai_key.lock().unwrap().clone();
                    if openai_key.is_empty() {
                        handle.emit("recording-state", "idle").ok();
                        handle.emit("error-message", "OpenAI API key not set").ok();
                        set_overlay_visible(&handle, false);
                        continue;
                    }

                    let anthropic_key = state.anthropic_key.lock().unwrap().clone();
                    tauri::async_runtime::spawn(process_recording(
                        handle.clone(),
                        openai_key,
                        anthropic_key,
                    ));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
