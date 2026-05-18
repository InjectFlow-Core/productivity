mod ai;
mod config;
mod plan;

use chrono::Local;
use fs2::FileExt;
use plan::{DayPlan, Priority};
use tauri::{LogicalSize, WebviewWindow};

// ── App mode ──────────────────────────────────────────────────────────────────

fn detect_mode() -> &'static str {
    let args: Vec<String> = std::env::args().collect();
    if args
        .iter()
        .any(|a| a == "--mode=sticky" || a == "--sticky")
    {
        "sticky"
    } else if args
        .iter()
        .any(|a| a == "--mode=evening" || a == "--evening")
    {
        "evening"
    } else if args
        .iter()
        .any(|a| a == "--mode=dashboard" || a == "--dashboard")
    {
        "dashboard"
    } else {
        "morning"
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AppState {
    mode: String,
    today_plan: Option<DayPlan>,
}

#[tauri::command]
fn get_app_state() -> AppState {
    AppState {
        mode: detect_mode().to_string(),
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
fn get_today_plan() -> Option<DayPlan> {
    plan::get_today_plan()
}

#[tauri::command]
fn get_carryover_candidates() -> Vec<Priority> {
    plan::get_yesterday_unfinished_priorities()
}

#[tauri::command]
fn toggle_priority(date: String, index: usize) -> Result<DayPlan, String> {
    plan::toggle_priority(&date, index)
}

#[tauri::command]
fn submit_plan(intention: String, priorities: Vec<Priority>, notes: String) -> Result<(), String> {
    let non_empty: Vec<Priority> = priorities
        .into_iter()
        .filter(|p| !p.text.trim().is_empty())
        .map(|p| Priority {
            text: p.text.trim().to_string(),
            done: false,
            category: p.category,
            carried_from: p.carried_from,
        })
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

    plan::save_today_plan(&day_plan)
}

#[tauri::command]
fn submit_review(review_notes: String) -> Result<(), String> {
    plan::save_review(review_notes)
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
fn configure_sticky_window(window: WebviewWindow) -> Result<(), String> {
    window.set_fullscreen(false).map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(360.0, 520.0))
        .map_err(|e| e.to_string())?;
    window.set_min_size(Some(LogicalSize::new(320.0, 360.0))).ok();
    window.set_always_on_top(true).ok();
    window.center().ok();
    Ok(())
}

#[tauri::command]
fn configure_dashboard_window(window: WebviewWindow) -> Result<(), String> {
    window.set_always_on_top(false).ok();
    window.set_fullscreen(true).map_err(|e| e.to_string())
}

#[tauri::command]
fn close_app(window: WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run() {
    let lock_file = std::fs::File::create(std::env::temp_dir().join("daily-planner.lock"))
        .expect("could not create instance lock file");
    if lock_file.try_lock_exclusive().is_err() {
        eprintln!("[daily-planner] another instance is already running — exiting");
        std::process::exit(0);
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            get_today_date,
            get_current_time,
            get_all_plans,
            get_today_plan,
            get_carryover_candidates,
            toggle_priority,
            submit_plan,
            submit_review,
            get_config,
            save_settings,
            get_ai_review,
            check_timers,
            configure_sticky_window,
            configure_dashboard_window,
            close_app,
        ])
        .run(tauri::generate_context!())
        .expect("error running daily-planner");
}
