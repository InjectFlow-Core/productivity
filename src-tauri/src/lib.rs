mod ai;
mod config;
mod lock;
mod plan;

use chrono::Local;
use fs2::FileExt;
use plan::{DayPlan, Priority};
use std::sync::Mutex;
use tauri::{Manager, State, WebviewWindow, WindowEvent};

// ── App mode ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Morning,
    Evening,
    Dashboard,
}

fn detect_mode() -> AppMode {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mode=evening" || a == "--evening") {
        AppMode::Evening
    } else if args.iter().any(|a| a == "--mode=dashboard" || a == "--dashboard") {
        AppMode::Dashboard
    } else {
        AppMode::Morning
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct LockState {
    xid: Mutex<Option<u32>>,
    locked: Mutex<bool>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AppState {
    mode: String,
    today_plan: Option<DayPlan>,
}

#[tauri::command]
fn get_app_state() -> AppState {
    let mode = match detect_mode() {
        AppMode::Morning => "morning",
        AppMode::Evening => "evening",
        AppMode::Dashboard => "dashboard",
    };
    AppState {
        mode: mode.to_string(),
        today_plan: plan::get_today_plan(),
    }
}

#[tauri::command]
fn get_today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

#[tauri::command]
fn get_current_time() -> String {
    Local::now().format("%H:%M").to_string()
}

#[tauri::command]
fn get_all_plans() -> Vec<DayPlan> {
    plan::get_all_plans()
}

#[tauri::command]
fn toggle_priority(date: String, index: usize) -> Result<DayPlan, String> {
    plan::toggle_priority(&date, index)
}

#[tauri::command]
fn submit_plan(
    state: State<std::sync::Arc<LockState>>,
    intention: String,
    priorities: Vec<Priority>,
    notes: String,
) -> Result<(), String> {
    let non_empty: Vec<Priority> = priorities
        .into_iter()
        .filter(|p| !p.text.trim().is_empty())
        .map(|p| Priority { text: p.text.trim().to_string(), done: false, category: p.category })
        .collect();

    if non_empty.is_empty() {
        return Err("Add at least one task for today.".into());
    }

    let day_plan = DayPlan {
        date: Local::now().format("%Y-%m-%d").to_string(),
        intention: intention.trim().to_string(),
        priorities: non_empty,
        notes: notes.trim().to_string(),
        created_at: Local::now().to_rfc3339(),
        reviewed_at: None,
        review_notes: None,
    };

    plan::save_today_plan(&day_plan)?;
    cleanup_lock_after_submit(&state);
    Ok(())
}

#[tauri::command]
fn submit_review(
    state: State<std::sync::Arc<LockState>>,
    review_notes: String,
) -> Result<(), String> {
    plan::save_review(review_notes)?;
    cleanup_lock_after_submit(&state);
    Ok(())
}

#[tauri::command]
fn get_config() -> config::Config {
    config::read_config()
}

#[tauri::command]
fn save_settings(
    morning_time: String,
    evening_time: String,
    claude_api_key: String,
) -> Result<(), String> {
    let mut cfg = config::read_config();
    cfg.morning_time = morning_time.clone();
    cfg.evening_time = evening_time.clone();
    cfg.claude_api_key = if claude_api_key.trim().is_empty() {
        None
    } else {
        Some(claude_api_key.trim().to_string())
    };
    config::save_config(&cfg)?;
    config::apply_schedule(&morning_time, &evening_time)
}

#[tauri::command]
async fn get_ai_review() -> Result<String, String> {
    let cfg = config::read_config();
    let api_key = cfg
        .claude_api_key
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "Add your Claude API key in settings to enable AI review.".to_string())?;
    let plans = plan::get_week_plans();
    ai::get_week_review(&api_key, &plans).await
}

#[tauri::command]
fn check_timers() -> bool {
    config::timers_installed()
}

#[tauri::command]
fn close_app(window: WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn normalize_lock_window(state: &LockState) -> Result<(), String> {
    if let Some(xid) = *state.xid.lock().unwrap() {
        lock::normalize_window(xid)?;
    }
    Ok(())
}

fn cleanup_lock_after_submit(state: &LockState) {
    if let Err(e) = lock::release_input() {
        eprintln!("[lock] release after submit failed: {}", e);
    }
    if let Err(e) = normalize_lock_window(state) {
        eprintln!("[lock] normalize after submit failed: {}", e);
    }
    *state.locked.lock().unwrap() = false;
}

fn open_as_dashboard(window: &WebviewWindow) {
    window.set_fullscreen(false).ok();
    window.set_always_on_top(false).ok();
    window.set_decorations(true).ok();
    window.set_resizable(true).ok();
    window.set_minimizable(true).ok();
    window.set_maximizable(true).ok();
    window.set_closable(true).ok();
    window.set_skip_taskbar(false).ok();
    window.maximize().ok();
}

fn setup_and_grab(window: &WebviewWindow, state: std::sync::Arc<LockState>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    for attempt in 1..=5 {
        std::thread::sleep(std::time::Duration::from_millis(300 * attempt));
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Xlib(h) = handle.as_raw() {
                let xid = h.window as u32;

                if let Err(e) = lock::setup_window(xid) {
                    eprintln!("[lock] setup attempt {}: {}", attempt, e);
                    continue;
                }

                window.set_focus().ok();

                match lock::grab_input(xid) {
                    Ok(_) => {
                        *state.xid.lock().unwrap() = Some(xid);
                        eprintln!("[lock] setup + grab succeeded on attempt {}", attempt);
                        return;
                    }
                    Err(e) => eprintln!("[lock] grab attempt {}: {}", attempt, e),
                }
            }
        }
    }
    eprintln!("[lock] All attempts failed — running without hard lock");
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run() {
    // Prevent multiple simultaneous instances. The OS releases the lock when the
    // process exits, so there is no cleanup needed on normal or abnormal exit.
    let lock_file = std::fs::File::create(std::env::temp_dir().join("daily-planner.lock"))
        .expect("could not create instance lock file");
    if lock_file.try_lock_exclusive().is_err() {
        eprintln!("[daily-planner] another instance is already running — exiting");
        std::process::exit(0);
    }

    let lock_state = std::sync::Arc::new(LockState {
        xid: Mutex::new(None),
        locked: Mutex::new(true),
    });

    let lock_state_setup = lock_state.clone();

    tauri::Builder::default()
        .manage(lock_state)
        .setup(move |app| {
            let window = app.get_webview_window("main").unwrap();
            let mode = detect_mode();

            let needs_lock = match &mode {
                AppMode::Morning => !plan::today_plan_exists(),
                AppMode::Evening => plan::today_plan_exists() && !plan::today_reviewed(),
                AppMode::Dashboard => false,
            };

            if needs_lock {
                let w = window.clone();
                let s = lock_state_setup.clone();
                std::thread::spawn(move || setup_and_grab(&w, s));
            } else {
                open_as_dashboard(&window);
                // Not a lock — allow close from the start
                *lock_state_setup.locked.lock().unwrap() = false;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<LockState>();
                if *state.locked.lock().unwrap() {
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            get_today_date,
            get_current_time,
            get_all_plans,
            toggle_priority,
            submit_plan,
            submit_review,
            get_config,
            save_settings,
            get_ai_review,
            check_timers,
            close_app,
        ])
        .run(tauri::generate_context!())
        .expect("error running daily-planner");
}
